//! Positive autonomous scanner fallback through the public native Service API.
//! Height zero is exclusively this test's interval, not a product history start.
//! Owned loopback RPC serves the complete historical Litecoin testnet genesis.
//! Process supervision bounds this harness; it does not cancel production work
//! or establish a Service thread-join, public-provider, or full-consensus claim.
#![cfg(all(not(target_arch = "wasm32"), unix))]

use bitcoin::{
    consensus::{deserialize, serialize},
    hashes::{sha256, Hash},
    Block,
};
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use urma_chain::{decode, observation::Chain, pow::check_header_pow};
use urma_wire::index::Index;
use wire_app_core::{connection::SourceChoice, feed::SyncPhase};
use wire_app_engine::{
    config::{ArchiveSettings, Settings, TokenAccess},
    native::service::Service,
};

const GENESIS: &str = "4966625a4b2851d9fdee139e56211a0d88575f59ed816ff5e6a63deb4e3e29a0";
const MERKLE: &str = "97ddfbbae6be97fd6cdf3e7ca13232a3afff2353e29badfab7f73011edd4ced9";
const CORPUS_SHA256: &str = "11fc8940b3dcf6ce81df69e393d56a98c466bed4c3ef5bfd3c9c341db966a0f1";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const COOKIE: &str = "fixture:one";
const BASIC_AUTH: &str = "Basic Zml4dHVyZTpvbmU=";
const IO_BUDGET: Duration = Duration::from_secs(3);
const CLEANUP_BUDGET: Duration = Duration::from_secs(5);
const SYNC_BUDGET: Duration = Duration::from_secs(12);
const HOST_BUDGET: Duration = Duration::from_secs(45);

// Complete 280-byte block, not a generated coinbase/header pair. Provenance:
// public litecoin =0.32.8-rc.1, src/blockdata/constants.rs:
// GENESIS_OUTPUT_PK and bitcoin_genesis_tx (lines 53-101), exact script hex in
// bitcoin_genesis_first_transaction (253-265), and testnet4 genesis (124-135).
// Its testnet4_genesis_full_block test fixes the historical header identity.
// The sibling urma-chain/tests/pow.rs checks that header's Litecoin scrypt PoW.
// No registry path, additional dependency, mining, MWEB suffix, or download is
// used here. This literal contains the original coinbase and fixed nonce.
const FULL_GENESIS_HEX: &str = concat!(
    "010000000000000000000000000000000000000000000000000000000000000000000000d9ced4ed",
    "1130f7b7faad9be25323ffafa33232a17c3edf6cfd97bee6bafbdd97f60ba158f0ff0f1ee1790400",
    "0101000000010000000000000000000000000000000000000000000000000000000000000000ffff",
    "ffff4804ffff001d0104404e592054696d65732030352f4f63742f32303131205374657665204a6f",
    "62732c204170706c65e280997320566973696f6e6172792c2044696573206174203536ffffffff01",
    "00f2052a010000004341040184710fa689ad5023690c80f3a49c8f13f8d45b8c857fbcbc8bc4a8e4",
    "d3eb4b10f4d4604fa08dce601aaf0f470216fe1b51850b4acf21b179c45070ac7b03a9ac00000000",
);

fn real_genesis() -> Vec<u8> {
    let raw = hex::decode(FULL_GENESIS_HEX).unwrap();
    assert_eq!(raw.len(), 280);
    assert_eq!(sha256::Hash::hash(&raw).to_string(), CORPUS_SHA256);
    let expected = Chain::LitecoinTestnet.genesis().unwrap().0;
    assert_eq!(expected.to_string(), GENESIS);
    let block: Block = deserialize(&raw).unwrap();
    assert_eq!(serialize(&block), raw);
    assert_eq!(block.block_hash(), expected);
    assert_eq!(block.header.merkle_root.to_string(), MERKLE);
    assert_eq!(block.header.prev_blockhash.to_byte_array(), [0; 32]);
    assert_eq!(block.header.time, 1_486_949_366);
    assert_eq!(block.header.bits.to_consensus(), 0x1e0f_fff0);
    assert_eq!(block.header.nonce, 293_345);
    assert_eq!(block.txdata.len(), 1);
    let coinbase = &block.txdata[0];
    assert!(coinbase.is_coinbase());
    assert_eq!(serialize(coinbase).len(), 199);
    assert_eq!(coinbase.compute_txid().to_string(), MERKLE);
    assert_eq!(coinbase.output.len(), 1);
    assert_eq!(coinbase.output[0].value.to_sat(), 5_000_000_000);
    assert!(block.check_merkle_root());
    check_header_pow(&block.header, Chain::LitecoinTestnet).unwrap();
    // Match Node::block_at's strict Core decoder, retaining complete body bytes.
    let core = decode::block_with_transactions(&raw, Chain::LitecoinTestnet, expected).unwrap();
    assert_eq!(core.block(), &block);
    assert_eq!(core.raw_transactions(), &[serialize(coinbase)]);
    assert_eq!(serialize(core.block()), raw);
    raw
}

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Archive {
        authenticated: bool,
        request: String,
    },
    Rpc {
        authenticated: bool,
        method: String,
        params: Value,
    },
}

struct Request {
    line: String,
    authorization: String,
    body: Vec<u8>,
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "owned fixture request budget"))
}

fn read_request(stream: &mut TcpStream) -> io::Result<Request> {
    let deadline = Instant::now() + IO_BUDGET;
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() == 8192 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture header capacity",
            ));
        }
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let text = std::str::from_utf8(&header)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut lines = text.split("\r\n");
    let line = lines.next().unwrap().to_owned();
    let mut authorization = String::new();
    let mut length = 0usize;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "fixture malformed header")
        })?;
        if name.eq_ignore_ascii_case("authorization") {
            authorization = value.trim().into();
        }
        if name.eq_ignore_ascii_case("content-length") {
            length = value
                .trim()
                .parse()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
    }
    if length > 16384 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture body capacity",
        ));
    }
    let mut body = vec![0; length];
    let mut offset = 0;
    while offset < length {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        let count = stream.read(&mut body[offset..])?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        offset += count;
    }
    Ok(Request {
        line,
        authorization,
        body,
    })
}

fn respond(stream: &mut TcpStream, status: &str, body: &[u8]) -> io::Result<()> {
    stream.set_write_timeout(Some(IO_BUDGET))?;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

enum Endpoint {
    UnavailableArchive,
    GenesisRpc { raw: String },
}

impl Endpoint {
    fn serve(&self, stream: &mut TcpStream, events: &Mutex<Vec<Event>>) -> io::Result<()> {
        let request = read_request(stream)?;
        match self {
            Self::UnavailableArchive => {
                events.lock().unwrap().push(Event::Archive {
                    authenticated: request.authorization == format!("Bearer {TOKEN}"),
                    request: request.line,
                });
                respond(stream, "503 Service Unavailable", &[])
            }
            Self::GenesisRpc { raw } => {
                if request.line != "POST / HTTP/1.1" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unexpected RPC route",
                    ));
                }
                let rpc: Value = serde_json::from_slice(&request.body)?;
                let method = rpc["method"].as_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "RPC method missing")
                })?;
                let params = &rpc["params"];
                let authenticated = request.authorization == BASIC_AUTH;
                events.lock().unwrap().push(Event::Rpc {
                    authenticated,
                    method: method.into(),
                    params: params.clone(),
                });
                // Never synthesize later heights or accept wallet/publication RPC.
                let result = match (method, params) {
                    ("getblockhash", args) if *args == json!([0]) => Some(json!(GENESIS)),
                    ("getblockchaininfo", args) if *args == json!([]) => Some(json!({
                        "blocks":0, "headers":0, "initialblockdownload":false, "bestblockhash":GENESIS,
                    })),
                    ("getblock", args) if *args == json!([GENESIS, 0]) => Some(json!(raw)),
                    _ => None,
                };
                let response = match result.filter(|_| authenticated) {
                    Some(result) => json!({"result":result,"error":null,"id":rpc["id"]}),
                    None => {
                        json!({"result":null,"error":{"code":-32601,"message":"fixture refuses RPC"},"id":rpc["id"]})
                    }
                };
                respond(stream, "200 OK", &serde_json::to_vec(&response)?)
            }
        }
    }
}

// These join handles belong to test responders only. The public app Service
// detaches its workers and exposes no shutdown/join API.
struct Responder {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    errors: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<()>>,
}

impl Responder {
    fn start(endpoint: Endpoint, events: Arc<Mutex<Vec<Event>>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let stopped = stop.clone();
        let failed = errors.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if let Err(error) = endpoint.serve(&mut stream, &events) {
                            failed.lock().unwrap().push(error.to_string());
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        failed.lock().unwrap().push(error.to_string());
                        break;
                    }
                }
            }
        });
        Self {
            address,
            stop,
            errors,
            worker: Some(worker),
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        let Some(worker) = &self.worker else {
            return Ok(());
        };
        let deadline = Instant::now() + CLEANUP_BUDGET;
        while !worker.is_finished() {
            if Instant::now() >= deadline {
                return Err("owned responder exceeded cleanup budget; join not attempted".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| "owned responder panicked".to_owned())?;
        let errors = self.errors.lock().unwrap();
        if !errors.is_empty() {
            return Err(format!("owned responder errors: {errors:?}"));
        }
        Ok(())
    }
}

impl Drop for Responder {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("bounded test responder cleanup: {error}");
        }
    }
}

fn private_file(path: &Path, contents: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(contents).unwrap();
}

fn stage(directory: &Path, description: &str) {
    fs::write(directory.join("stage"), description).unwrap();
}

fn fallback(directory: &Path) {
    stage(
        directory,
        "checking complete public genesis, Merkle, scrypt PoW and strict Core decoding",
    );
    let raw = real_genesis();
    let app = directory.join("app");
    fs::create_dir(&app).unwrap();
    // Fixture interval explicitly selected; never consult/change history defaults.
    fs::write(app.join("ltc-testnet-start.json"), b"0").unwrap();
    let index_path = app.join("ltc-testnet-index-v3.json");
    assert!(!index_path.exists());
    let token = directory.join("archive.token");
    let cookie = directory.join("rpc.cookie");
    private_file(&token, TOKEN.as_bytes());
    private_file(&cookie, COOKIE.as_bytes());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut archive = Responder::start(Endpoint::UnavailableArchive, events.clone());
    let mut rpc = Responder::start(
        Endpoint::GenesisRpc {
            raw: hex::encode(&raw),
        },
        events.clone(),
    );
    let settings = Settings {
        rpc_url: Some(format!("http://{}/", rpc.address)),
        node_auth_file: Some(cookie),
        archive: Some(ArchiveSettings {
            endpoint: archive.address,
            token_file: token,
            access: TokenAccess::Owner,
        }),
        source: SourceChoice::LocalNode,
        cache_dir: app.join("runtime"),
        vault: None,
        unlock_file: None,
    };
    let service = Service::open(&app, settings.clone()).unwrap();
    stage(
        directory,
        "public Service start_sync: optional archive503 then explicit owned LocalNode RPC",
    );
    let started = service.start_sync().unwrap();
    assert_eq!(started.run, 1);
    assert_eq!(started.phase, SyncPhase::Connecting);
    let deadline = Instant::now() + SYNC_BUDGET;
    let complete = loop {
        let status = service.sync_snapshot();
        if status.settled() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "Service sync did not settle within harness poll budget: {status:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(complete.phase, SyncPhase::Complete);
    assert_eq!(complete.run, 1);
    assert_eq!((complete.start, complete.from, complete.tip), (0, 0, 0));
    assert_eq!(complete.indexed_to, Some(0));
    assert_eq!((complete.done, complete.total), (1, 1));
    assert!(complete.complete_to_tip);
    let feed = service.feed().unwrap();
    assert_eq!((feed.start, feed.tip), (0, 0));
    assert!(feed.complete_to_tip);
    assert!(feed.posts.is_empty() && feed.identities.is_empty());
    let index = Index::load(&index_path).unwrap();
    assert_eq!(index.format, Index::FORMAT);
    assert_eq!(index.genesis, GENESIS);
    assert_eq!(index.start, 0);
    assert_eq!(index.blocks.len(), 1);
    assert_eq!(index.blocks[0].height, 0);
    assert_eq!(index.blocks[0].hash, GENESIS);
    assert!(index.entries.is_empty());
    assert_eq!(fs::read(app.join("ltc-testnet-start.json")).unwrap(), b"0");
    let disk = fs::read(&index_path).unwrap();

    let rpc_event = |method: &str, params| Event::Rpc {
        authenticated: true,
        method: method.into(),
        params,
    };
    let expected = vec![
        Event::Archive {
            authenticated: true,
            request: "GET /v1/wire?limit=128 HTTP/1.1".into(),
        },
        rpc_event("getblockhash", json!([0])), // network identity
        rpc_event("getblockchaininfo", json!([])), // fixture tip
        rpc_event("getblockhash", json!([0])), // actual fetch
        rpc_event("getblock", json!([GENESIS, 0])), // complete real block
        rpc_event("getblockhash", json!([0])), // index tail confirmation
    ];
    assert_eq!(*events.lock().unwrap(), expected);
    stage(
        directory,
        "scanner completed one real checkpoint; reopening app to read unchanged disk index",
    );
    drop(service);
    // Successful reopen demonstrates release of the public Service owner lock.
    // Idle status is fresh; persisted checkpoints do not fabricate new coverage.
    let reopened = Service::open(&app, settings).unwrap();
    let idle = reopened.sync_snapshot();
    assert_eq!(idle.phase, SyncPhase::Idle);
    assert_eq!(idle.run, 0);
    assert_eq!(idle.indexed_to, None);
    assert!(!idle.complete_to_tip);
    let cached = reopened.feed().unwrap();
    assert_eq!((cached.start, cached.tip), (0, 0));
    assert!(cached.posts.is_empty() && cached.identities.is_empty());
    assert!(!cached.complete_to_tip);
    assert_eq!(fs::read(&index_path).unwrap(), disk);
    assert_eq!(Index::load(&index_path).unwrap().blocks[0].hash, GENESIS);
    assert_eq!(*events.lock().unwrap(), expected);
    drop(reopened);

    stage(
        directory,
        "joining owned test responders after normal completed scan and app release",
    );
    archive.finish().unwrap();
    rpc.finish().unwrap();
    let _archive_port = TcpListener::bind(archive.address).unwrap();
    let _rpc_port = TcpListener::bind(rpc.address).unwrap();
    fs::write(
        directory.join("result.json"),
        serde_json::to_vec(&json!({
            "real_genesis_checked":true, "scanner_checkpoints":1,
            "restart_disk_unchanged":true, "owned_responder_joins_returned":true,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "owned bounded subprocess host, invoked only by the active fallback test"]
fn archive_fallback_host() {
    let directory =
        PathBuf::from(std::env::var_os("U4_ARCHIVE_FALLBACK_TEMP").expect("owned TEMP host"));
    fallback(&directory);
}

struct Host {
    child: Option<Child>,
    directory: PathBuf,
}

impl Host {
    fn logs(&self) -> String {
        ["stage", "stdout", "stderr"]
            .map(|name| {
                let mut text = String::new();
                if let Ok(file) = File::open(self.directory.join(name)) {
                    let _ = file.take(64 * 1024).read_to_string(&mut text);
                }
                text
            })
            .join("\n")
    }

    fn finish(mut self) {
        let deadline = Instant::now() + HOST_BUDGET;
        loop {
            if let Some(exit) = self.child.as_mut().unwrap().try_wait().unwrap() {
                self.child.take();
                assert!(
                    exit.success(),
                    "fallback host failed: {exit}; {}",
                    self.logs()
                );
                let report: Value =
                    serde_json::from_slice(&fs::read(self.directory.join("result.json")).unwrap())
                        .unwrap();
                assert_eq!(
                    report,
                    json!({
                        "real_genesis_checked":true, "scanner_checkpoints":1,
                        "restart_disk_unchanged":true, "owned_responder_joins_returned":true,
                    })
                );
                return;
            }
            assert!(Instant::now() < deadline,
                "fallback host exceeded45s; kill/reap is abnormal test cleanup, not Service cancellation/join; {}", self.logs());
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = child.kill();
        let deadline = Instant::now() + CLEANUP_BUDGET;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                result => {
                    eprintln!("fallback host kill/reap cleanup exceeded budget: {result:?}");
                    return;
                }
            }
        }
    }
}

#[test]
fn unavailable_archive_falls_back_to_real_rpc_scan_and_persists_checkpoint_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "archive_fallback_host",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("U4_ARCHIVE_FALLBACK_TEMP", directory.path())
        .env("TMPDIR", directory.path())
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            File::create(directory.path().join("stdout")).unwrap(),
        ))
        .stderr(Stdio::from(
            File::create(directory.path().join("stderr")).unwrap(),
        ))
        .spawn()
        .unwrap();
    // Child is guarded before assertions; TEMP outlives normal/abnormal cleanup.
    Host {
        child: Some(child),
        directory: directory.path().to_owned(),
    }
    .finish();
}
