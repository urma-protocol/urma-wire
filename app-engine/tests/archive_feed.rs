//! Public app Service integration over owned loopback/TEMP archives.
//! Fixtures retain genuine signatures and witness inclusion; the in-memory
//! branch selector does not establish PoW, scripts, UTXOs or consensus.
//! Gates control response publication, not thread scheduling or hard cancellation.
#![cfg(not(target_arch = "wasm32"))]

use bitcoin::{
    block::Header,
    blockdata::constants::genesis_block,
    consensus::{deserialize, serialize},
    hashes::Hash,
    Amount, Block, BlockHash, CompactTarget, Network, ScriptBuf, Transaction, Witness,
};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use urma_chain::{
    inclusion::TransactionProof, observation::Chain, validation::validate_block_integrity,
};
use urma_core::{
    format::{PublicRecord, RecordKind},
    multipart::VerifiedRecord,
};
use urma_node::{
    engine::Engine,
    model::{ArchiveConfig, ChainPoint, RecordPackage, Retention, API_VERSION},
    service::Service as NodeService,
    source::ChainSource,
    Error as NodeError,
};
use urma_node_client::{
    projection::{WireCursor, WirePage},
    Client,
};
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_core::{
    connection::SourceChoice,
    feed::{Lookup, SyncPhase, SyncStatus},
    model::{Placement, Txid},
};
use wire_app_engine::{
    config::{ArchiveSettings, Settings, NETWORK},
    errors::NativeError,
    native::service::Service,
    projection,
};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const BOUND: Duration = Duration::from_secs(4);
const HOST_BOUND: Duration = Duration::from_secs(45);
const START: u64 = 100;
const INDEX_FILE: &str = "ltc-testnet-index.json";
const START_FILE: &str = "ltc-testnet-start.json";

fn stage(root: &Path, message: &str) {
    fs::write(root.join("stage"), message).unwrap();
}
fn json_feed(service: &Service) -> Value {
    serde_json::to_value(service.feed().unwrap()).unwrap()
}
fn app_id(pair: &Pair) -> Txid {
    Txid::from_display_hex(&pair.reveal.compute_txid().to_string()).unwrap()
}

struct Pair {
    commit: Transaction,
    reveal: Transaction,
    verified: VerifiedRecord,
}
impl Pair {
    fn load(name: &str) -> Self {
        // Portable sibling public vectors: no private or machine-specific path.
        let vectors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../urma/tests/vectors");
        let raw_commit = fs::read(vectors.join(format!("{name}.commit"))).unwrap();
        let raw_reveal = fs::read(vectors.join(format!("{name}.reveal"))).unwrap();
        let commit: Transaction = deserialize(&raw_commit).unwrap();
        let reveal: Transaction = deserialize(&raw_reveal).unwrap();
        assert_eq!(serialize(&commit), raw_commit);
        assert_eq!(serialize(&reveal), raw_reveal);
        let verified = VerifiedRecord::verify(reveal.compute_txid(), &reveal, &commit).unwrap();
        Self {
            commit,
            reveal,
            verified,
        }
    }
    fn record(&self) -> PublicRecord {
        PublicRecord::decode(self.verified.record_bytes()).unwrap()
    }
    fn transactions(&self) -> [Transaction; 2] {
        [self.commit.clone(), self.reveal.clone()]
    }
}
fn block(transactions: &[Transaction], previous: Option<&Block>, nonce: u32) -> Block {
    let mut block = genesis_block(Network::Regtest);
    if let Some(previous) = previous {
        block.header.prev_blockhash = previous.block_hash();
    }
    block.header.bits = CompactTarget::from_consensus(0);
    block.header.nonce = nonce;
    block.txdata[0].input[0].witness = Witness::from_slice(&[[0; 32]]);
    block.txdata.extend_from_slice(transactions);
    let commitment = Block::compute_witness_commitment(&block.witness_root().unwrap(), &[0; 32]);
    let mut script = vec![0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed];
    script.extend_from_slice(commitment.as_byte_array());
    block.txdata[0].output.push(bitcoin::TxOut {
        value: Amount::ZERO,
        script_pubkey: ScriptBuf::from_bytes(script),
    });
    block.header.merkle_root = block.compute_merkle_root().unwrap();
    validate_block_integrity(&block, block.block_hash()).unwrap();
    // Transparent witness encoding also passes the strict Litecoin decoder.
    urma_chain::decode::block(&serialize(&block), NETWORK, block.block_hash()).unwrap();
    block
}
fn point(height: u64, block: &Block) -> ChainPoint {
    ChainPoint {
        height,
        hash: block.block_hash(),
    }
}
struct Source {
    branch: Mutex<Vec<Block>>,
    calls: AtomicUsize,
    bodies: AtomicUsize,
}
impl Source {
    fn selected(&self, height: u64) -> Result<Block, NodeError> {
        self.branch
            .lock()
            .unwrap()
            .get(height.checked_sub(START).ok_or(NodeError::Stale)? as usize)
            .cloned()
            .ok_or(NodeError::Stale)
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}
impl ChainSource for Source {
    fn genesis(&self) -> Result<BlockHash, NodeError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NETWORK.genesis().unwrap().0)
    }
    fn tip(&self) -> Result<ChainPoint, NodeError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let branch = self.branch.lock().unwrap();
        Ok(point(
            START + branch.len() as u64 - 1,
            branch.last().unwrap(),
        ))
    }
    fn header(&self, height: u64) -> Result<Header, NodeError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.selected(height)?.header)
    }
    fn block(&self, height: u64) -> Result<Vec<u8>, NodeError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.bodies.fetch_add(1, Ordering::Relaxed);
        Ok(serialize(&self.selected(height)?))
    }
}
struct Fixture {
    profile: Pair,
    post: Pair,
    reply: Pair,
    blocks: Vec<Block>,
    source: Arc<Source>,
    engine: Arc<Engine>,
}
impl Fixture {
    fn new(root: &Path) -> Self {
        let profile = Pair::load("proof-profile-utf8-two-byte128");
        let post = Pair::load("proof-post");
        let reply = Pair::load("proof-reply");
        let root_block = block(
            &[profile.transactions(), post.transactions()].concat(),
            None,
            8111,
        );
        let tail = block(&reply.transactions(), Some(&root_block), 8112);
        let blocks = vec![root_block, tail];
        let source = Arc::new(Source {
            branch: Mutex::new(blocks.clone()),
            calls: AtomicUsize::new(0),
            bodies: AtomicUsize::new(0),
        });
        let engine = Arc::new(
            Engine::open(
                &root.join("archive"),
                ArchiveConfig {
                    chain: NETWORK,
                    application_start: point(START, &blocks[0]),
                    retention: Retention::Unpruned,
                },
                Some(source.clone()),
            )
            .unwrap(),
        );
        engine.synchronize_headers().unwrap();
        for (pair, index, position) in [(&profile, 0, 2), (&post, 0, 4), (&reply, 1, 2)] {
            engine
                .admit_record(RecordPackage {
                    version: API_VERSION,
                    chain_genesis: NETWORK.genesis().unwrap().0,
                    block: point(START + index as u64, &blocks[index]),
                    reveal: TransactionProof::from_block(
                        &blocks[index],
                        position,
                        serialize(&pair.reveal),
                        serialize(&blocks[index].txdata[0]),
                    )
                    .unwrap(),
                    commit: serialize(&pair.commit),
                })
                .unwrap();
        }
        assert_eq!(engine.status().unwrap().scanned_blocks, 0);
        assert!(engine.status().unwrap().indexed_through.is_none());
        assert_eq!(source.bodies.load(Ordering::Relaxed), 0);
        Self {
            profile,
            post,
            reply,
            blocks,
            source,
            engine,
        }
    }
    fn reply_target(&self) -> Txid {
        match self.reply.record() {
            PublicRecord::Reply { target, .. } => wire_app_engine::record::wire_txid(target),
            other => panic!("unexpected signed fixture {other:?}"),
        }
    }
    fn check(&self, service: &Service, with_reply: bool, complete: bool) {
        let feed = service.feed().unwrap();
        assert_eq!(
            (feed.start, feed.tip, feed.complete_to_tip),
            (START, START + 1, complete)
        );
        let nickname = match self.profile.record() {
            PublicRecord::Profile(name) => name,
            _ => unreachable!(),
        };
        assert_eq!(
            feed.identities[&self.profile.verified.author().to_string()]
                .nickname
                .as_deref(),
            Some(nickname.as_str())
        );
        assert_eq!(feed.posts.len(), if with_reply { 2 } else { 1 });
        for (pair, height, position, placement) in [
            (&self.post, START, 4, Placement::Root),
            (
                &self.reply,
                START + 1,
                2,
                Placement::Reply(self.reply_target()),
            ),
        ] {
            let lookup = service.post(app_id(pair)).unwrap();
            if height != START && !with_reply {
                assert!(matches!(lookup, Lookup::Absent));
                continue;
            }
            let Lookup::Found(found) = lookup else {
                panic!("missing signed fixture post")
            };
            let text = match pair.record() {
                PublicRecord::Post(text) | PublicRecord::Reply { text, .. } => text,
                _ => unreachable!(),
            };
            assert_eq!(found.txid, app_id(pair));
            assert_eq!(found.author.to_string(), pair.verified.author().to_string());
            assert_eq!(found.text, text);
            assert_eq!(found.placement, placement);
            assert_eq!(found.status.block_height, height);
            assert_eq!(found.status.tx_index, position);
            assert_eq!(
                found.status.block_hash,
                self.blocks[(height - START) as usize]
                    .block_hash()
                    .to_string()
            );
            assert_eq!(found.status.confirmations, START + 2 - height);
            assert_eq!(
                feed.posts.iter().find(|post| post.txid == found.txid),
                Some(&found)
            );
        }
        let replies = service.replies(self.reply_target()).unwrap();
        assert_eq!(replies.len(), usize::from(with_reply));
        if with_reply {
            assert_eq!(replies[0].txid, app_id(&self.reply));
        }
        assert!(matches!(
            service.post(Txid::from_raw_bytes([0xff; 32])).unwrap(),
            Lookup::Absent
        ));
    }
}

fn http_shutdown(address: SocketAddr) {
    let mut stream = TcpStream::connect_timeout(&address, BOUND).unwrap();
    stream.set_read_timeout(Some(BOUND)).unwrap();
    stream.set_write_timeout(Some(BOUND)).unwrap();
    write!(stream, "POST /v1/shutdown HTTP/1.1\r\nHost: loopback\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = Vec::new();
    stream.take(8192).read_to_end(&mut response).unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
    assert!(response.contains("\"stopping\":true"));
}
fn join_worker<T>(worker: JoinHandle<T>) -> T {
    let deadline = Instant::now() + BOUND;
    while !worker.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        worker.is_finished(),
        "worker did not join within harness bound; host supervisor owns abnormal cleanup"
    );
    worker.join().unwrap_or_else(|_| panic!("worker panicked"))
}
struct Server {
    address: SocketAddr,
    worker: Option<JoinHandle<Result<(), NodeError>>>,
}
impl Server {
    fn new(engine: Arc<Engine>) -> Self {
        let service =
            NodeService::bind("127.0.0.1:0".parse().unwrap(), TOKEN.into(), engine).unwrap();
        let address = service.address().unwrap();
        Self {
            address,
            worker: Some(thread::spawn(move || service.run(false))),
        }
    }
    fn client(&self) -> Client {
        Client::new(self.address, TOKEN.into()).unwrap()
    }
    fn close(&mut self) {
        if let Some(worker) = self.worker.take() {
            http_shutdown(self.address);
            join_worker(worker).unwrap();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.close();
    }
}

struct Incoming {
    cursor: Option<WireCursor>,
    response: SyncSender<(u16, Vec<u8>)>,
}
struct Relay {
    address: SocketAddr,
    incoming: Receiver<Incoming>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    calls: Arc<AtomicUsize>,
}
impl Relay {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let counting = calls.clone();
        let (incoming_tx, incoming) = mpsc::channel();
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("relay accept: {error}"),
                };
                stream.set_read_timeout(Some(BOUND)).unwrap();
                stream.set_write_timeout(Some(BOUND)).unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    assert!(header.len() < 8192);
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                }
                let header = String::from_utf8(header).unwrap();
                assert!(header.contains(&format!("Authorization: Bearer {TOKEN}\r\n")));
                let line = header.lines().next().unwrap();
                let query = line
                    .strip_prefix("GET /v1/wire?")
                    .unwrap()
                    .strip_suffix(" HTTP/1.1")
                    .unwrap();
                let mut fields = query.split('&');
                assert_eq!(fields.next(), Some("limit=128"));
                let cursor = fields.next().map(|field| {
                    serde_json::from_slice::<WireCursor>(
                        &hex::decode(field.strip_prefix("cursor=").unwrap()).unwrap(),
                    )
                    .unwrap()
                });
                assert!(fields.next().is_none());
                counting.fetch_add(1, Ordering::Relaxed);
                let (response, received) = mpsc::sync_channel(1);
                incoming_tx.send(Incoming { cursor, response }).unwrap();
                let (code, body) = received
                    .recv_timeout(BOUND)
                    .expect("test did not release relay response gate");
                assert!(body.len() <= 1024 * 1024);
                write!(stream, "HTTP/1.1 {code} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        Self {
            address,
            incoming,
            stop,
            worker: Some(worker),
            calls,
        }
    }
    fn next(&self) -> Incoming {
        self.incoming
            .recv_timeout(BOUND)
            .expect("app did not reach response gate")
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            join_worker(worker);
        }
    }
}
impl Incoming {
    fn json(self, value: Value) {
        self.response
            .send((200, serde_json::to_vec(&value).unwrap()))
            .unwrap();
    }
    fn page(self, page: WirePage) {
        self.json(serde_json::to_value(page).unwrap());
    }
    fn backend(&self, server: &Server, limit: u32) -> Result<WirePage, urma_node_client::Error> {
        server.client().wire_page(self.cursor.as_ref(), limit)
    }
}

fn settings(root: &Path, endpoint: Option<SocketAddr>) -> Settings {
    let token = root.join("archive.token");
    fs::write(&token, TOKEN).unwrap();
    fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
    Settings {
        source: SourceChoice::LocalNode,
        cache_dir: root.join("runtime"),
        archive: endpoint.map(|endpoint| ArchiveSettings {
            endpoint,
            token_file: token,
        }),
        ..Settings::default()
    }
}
fn open_app(root: &Path, endpoint: Option<SocketAddr>) -> Service {
    Service::open(&root.join("app"), settings(root, endpoint)).unwrap()
}
fn no_autonomous_files(root: &Path) {
    assert!(!root.join("runtime").exists());
    for name in [START_FILE, INDEX_FILE, "source.json"] {
        assert!(
            !root.join("app").join(name).exists(),
            "healthy archive created {name}"
        );
    }
}
fn settled(service: &Service) -> SyncStatus {
    let deadline = Instant::now() + BOUND;
    loop {
        let status = service.sync_snapshot();
        if status.settled() {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "app worker exceeded bounded wait: {status:?}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
fn begin(service: &Service) {
    let previous = service.sync_snapshot().run;
    let status = service.start_sync().unwrap();
    assert_eq!(status.run, previous + 1);
    assert_eq!(status.phase, SyncPhase::Connecting);
    assert!(!status.complete_to_tip);
}
fn complete(service: &Service, indexed: Option<u64>, done: u64, full: bool) {
    let status = settled(service);
    assert_eq!(status.phase, SyncPhase::Complete);
    assert_eq!(
        (
            status.start,
            status.tip,
            status.indexed_to,
            status.done,
            status.total,
            status.complete_to_tip
        ),
        (START, START + 1, indexed, done, 2, full)
    );
}
fn fallback_refused(service: &Service) {
    let status = settled(service);
    let SyncPhase::Failed(reason) = status.phase else {
        panic!("expected existing fallback refusal: {status:?}")
    };
    assert!(reason.contains("rpc_url and node_auth_file"));
    assert!(!status.complete_to_tip);
    assert!(service.source_status().endpoints.is_empty());
}
fn serve_full(relay: &Relay, server: &Server) {
    let incoming = relay.next();
    assert!(incoming.cursor.is_none());
    let page = incoming.backend(server, 128).unwrap();
    assert!(page.next.is_none());
    incoming.page(page);
}
fn serve_paged(relay: &Relay, server: &Server) {
    let mut previous = None;
    loop {
        let incoming = relay.next();
        assert_eq!(incoming.cursor, previous);
        let page = incoming.backend(server, 1).unwrap();
        previous = page.next.clone();
        incoming.page(page);
        if previous.is_none() {
            return;
        }
    }
}

fn healthy(root: &Path) {
    let fixture = Fixture::new(root);
    let mut server = Server::new(fixture.engine.clone());
    let service = open_app(root, Some(server.address));
    assert_eq!(service.sync_snapshot(), SyncStatus::idle());
    assert!(service.feed().unwrap().posts.is_empty());
    no_autonomous_files(root);
    let calls = fixture.source.calls();
    let node_status = serde_json::to_value(fixture.engine.status().unwrap()).unwrap();
    begin(&service);
    complete(&service, None, 0, false);
    fixture.check(&service, true, false);
    no_autonomous_files(root);
    assert_eq!(fixture.source.calls(), calls);
    assert_eq!(
        serde_json::to_value(fixture.engine.status().unwrap()).unwrap(),
        node_status
    );
    assert_eq!(service.source_status().choice, SourceChoice::LocalNode);
    assert!(!service.source_status().local_node_configured);
    // Archive results are memory-only: reopening does not create a local index.
    drop(service);
    let service = open_app(root, Some(server.address));
    assert!(service.feed().unwrap().posts.is_empty());
    begin(&service);
    complete(&service, None, 0, false);
    fixture.check(&service, true, false);
    assert_eq!(fixture.source.calls(), calls);
    no_autonomous_files(root);
    drop(service);
    server.close();
    assert_eq!(Arc::strong_count(&fixture.engine), 1);
}
fn refresh(root: &Path) {
    let fixture = Fixture::new(root);
    fixture.engine.scan_next().unwrap();
    fixture.engine.scan_next().unwrap();
    let mut server = Server::new(fixture.engine.clone());
    let relay = Relay::new();
    let service = open_app(root, Some(relay.address));
    let calls = fixture.source.calls();
    begin(&service);
    serve_full(&relay, &server);
    complete(&service, Some(START + 1), 2, true);
    fixture.check(&service, true, true);
    assert_eq!(fixture.source.calls(), calls);
    let prior = json_feed(&service);
    let replacement = block(&[], Some(&fixture.blocks[0]), 8113);
    *fixture.source.branch.lock().unwrap() = vec![fixture.blocks[0].clone(), replacement];
    fixture.engine.synchronize_headers().unwrap();
    let calls = fixture.source.calls();
    begin(&service);
    let gate = relay.next();
    assert!(gate.cursor.is_none());
    assert_eq!(service.sync_snapshot().phase, SyncPhase::Connecting);
    let mut during = json_feed(&service);
    assert_eq!(during["complete_to_tip"], false);
    during["complete_to_tip"] = true.into();
    assert_eq!(
        during, prior,
        "gated refresh published new records before response"
    );
    let selected = gate.backend(&server, 128).unwrap();
    assert_eq!(selected.records.len(), 2);
    assert_eq!(selected.archive.indexed_through.unwrap().height, START);
    gate.page(selected);
    complete(&service, Some(START), 1, false);
    fixture.check(&service, false, false);
    assert_eq!(fixture.source.calls(), calls);
    let incomplete = json_feed(&service);
    fixture.engine.scan_next().unwrap();
    let calls = fixture.source.calls();
    begin(&service);
    let gate = relay.next();
    assert_eq!(
        json_feed(&service),
        incomplete,
        "old sparse publication claimed completion before replacement arrived"
    );
    let selected = gate.backend(&server, 128).unwrap();
    assert_eq!(
        selected.archive.indexed_through,
        selected.archive.selected_tip
    );
    gate.page(selected);
    complete(&service, Some(START + 1), 2, true);
    fixture.check(&service, false, true);
    let mut completed = json_feed(&service);
    completed["complete_to_tip"] = false.into();
    assert_eq!(
        completed, incomplete,
        "records and coverage were mixed across refresh publications"
    );
    assert_eq!(fixture.source.calls(), calls);
    no_autonomous_files(root);
    // Gates establish before/after publication states, not a scheduler-race proof.
    drop(service);
    drop(relay);
    server.close();
    assert_eq!(Arc::strong_count(&fixture.engine), 1);
}

fn old_cache(root: &Path) -> (Value, Vec<u8>, Vec<u8>) {
    let app = root.join("app");
    fs::create_dir_all(&app).unwrap();
    let author = Pair::load("proof-post").verified.author().to_string();
    let record = |position, txid: u8, raw: Vec<u8>| Entry {
        height: 90,
        position,
        txid: format!("{txid:064x}"),
        author: author.clone(),
        record: hex::encode(raw),
    };
    let mut legacy_profile = RecordKind::Profile.prefix().to_vec();
    legacy_profile.extend_from_slice(&[b'l'; 129]);
    let mut legacy_avatar = RecordKind::Avatar.prefix().to_vec();
    legacy_avatar.extend_from_slice(&[0x12; 512]);
    let index = Index {
        format: "URMA-WIRE-INDEX-2".into(),
        genesis: NETWORK.genesis().unwrap().0.to_string(),
        start: 90,
        blocks: vec![Checkpoint {
            height: 90,
            hash: "09".repeat(32),
        }],
        entries: vec![
            record(
                0,
                1,
                PublicRecord::Post("old local cache".into())
                    .encode()
                    .unwrap(),
            ),
            record(
                1,
                2,
                PublicRecord::Profile("old name".into()).encode().unwrap(),
            ),
            record(2, 3, legacy_profile),
            record(
                3,
                4,
                PublicRecord::Avatar(Box::new([0x34; 128]))
                    .encode()
                    .unwrap(),
            ),
            record(4, 5, legacy_avatar),
        ],
    };
    index.persist(&app.join(INDEX_FILE)).unwrap();
    fs::write(app.join(START_FILE), "90\n").unwrap();
    (
        serde_json::to_value(projection::project(&index, false).unwrap()).unwrap(),
        fs::read(app.join(INDEX_FILE)).unwrap(),
        fs::read(app.join(START_FILE)).unwrap(),
    )
}
fn cache_unchanged(root: &Path, index: &[u8], start: &[u8]) {
    assert_eq!(fs::read(root.join("app").join(INDEX_FILE)).unwrap(), index);
    assert_eq!(fs::read(root.join("app").join(START_FILE)).unwrap(), start);
    assert!(!root.join("runtime").exists());
}
fn traversal(root: &Path) {
    let fixture = Fixture::new(root);
    let mut server = Server::new(fixture.engine.clone());
    let relay = Relay::new();
    let (old, index, start) = old_cache(root);
    let service = open_app(root, Some(relay.address));
    assert_eq!(json_feed(&service), old);
    for failure in 0..5 {
        stage(root, &format!("traversal fault {failure}"));
        // An initial attempt has no archive view; subsequent attempts prove an
        // already successful archive publication is discarded after failure.
        if failure != 0 {
            begin(&service);
            serve_paged(&relay, &server);
            complete(
                &service,
                if failure > 1 { Some(START) } else { None },
                if failure > 1 { 1 } else { 0 },
                false,
            );
            fixture.check(&service, true, false);
        }
        let previous = json_feed(&service);
        let calls = fixture.source.calls();
        let requests = relay.calls();
        begin(&service);
        let first = relay.next();
        assert!(first.cursor.is_none());
        let page = first.backend(&server, 1).unwrap();
        assert_eq!(page.records.len(), 1);
        assert!(page.next.is_some());
        first.page(page);
        let second = relay.next();
        assert!(second.cursor.is_some());
        let mut expected_during = previous.clone();
        expected_during["complete_to_tip"] = false.into();
        assert_eq!(
            json_feed(&service),
            expected_during,
            "first page escaped before terminal page"
        );
        match failure {
            0 => second
                .response
                .send((
                    500,
                    serde_json::to_vec(&json!({"error":"fixture unavailable"})).unwrap(),
                ))
                .unwrap(),
            1 => {
                fixture.engine.scan_next().unwrap();
                assert!(
                    matches!(
                        second.backend(&server, 1),
                        Err(urma_node_client::Error::Busy(_))
                    ),
                    "real Node must reject the now-stale cursor"
                );
                second
                    .response
                    .send((
                        409,
                        serde_json::to_vec(&json!({"error":"stale fixture snapshot"})).unwrap(),
                    ))
                    .unwrap();
            }
            2 => second.response.send((200, b"{".to_vec())).unwrap(),
            3 | 4 => {
                let page = second.backend(&server, 1).unwrap();
                let mut value = serde_json::to_value(page).unwrap();
                if failure == 3 {
                    value["archive"]["chain_genesis"] =
                        json!(Chain::BitcoinRegtest.genesis().unwrap().0);
                } else {
                    let count = value["archive"]["stored_transactions"].as_u64().unwrap();
                    value["archive"]["stored_transactions"] = (count + 1).into();
                }
                second.json(value);
            }
            _ => unreachable!(),
        }
        fallback_refused(&service);
        assert_eq!(
            json_feed(&service),
            old,
            "failed traversal published a partial/stale archive view"
        );
        assert_eq!(
            relay.calls(),
            requests + 2,
            "unexpected retry or further page"
        );
        if failure != 1 {
            assert_eq!(fixture.source.calls(), calls);
        }
        assert!(matches!(
            service.post(app_id(&fixture.post)).unwrap(),
            Lookup::Absent
        ));
        assert!(service.replies(fixture.reply_target()).unwrap().is_empty());
        cache_unchanged(root, &index, &start);
    }
    drop(service);
    drop(relay);
    server.close();
    assert_eq!(Arc::strong_count(&fixture.engine), 1);
}
fn disabled(root: &Path) {
    let relay = Relay::new();
    let (old, index, start) = old_cache(root);
    for _ in 0..2 {
        let service = open_app(root, None);
        assert_eq!(json_feed(&service), old);
        let txid = Txid::from_display_hex(&format!("{:064x}", 1)).unwrap();
        let Lookup::Found(post) = service.post(txid).unwrap() else {
            panic!("old cached post missing")
        };
        assert_eq!(post.text, "old local cache");
        assert!(service.replies(txid).unwrap().is_empty());
        assert!(matches!(
            service.start_sync(),
            Err(NativeError::LocalNodeUnconfigured)
        ));
        assert_eq!(service.sync_snapshot(), SyncStatus::idle());
        assert_eq!(json_feed(&service), old);
        drop(service);
        cache_unchanged(root, &index, &start);
    }
    assert_eq!(relay.calls(), 0);
    drop(relay);
}

#[test]
#[ignore = "owned subprocess host for bounded archive-feed integration cases"]
fn archive_feed_host() {
    let root =
        PathBuf::from(std::env::var_os("U3_ARCHIVE_FEED_TEMP").expect("host requires owned TEMP"));
    let scenario = std::env::var("U3_ARCHIVE_FEED_CASE").unwrap();
    stage(&root, &scenario);
    match scenario.as_str() {
        "healthy" => healthy(&root),
        "refresh" => refresh(&root),
        "traversal" => traversal(&root),
        "disabled" => disabled(&root),
        _ => panic!("unknown owned case"),
    }
    fs::write(
        root.join("receipt.json"),
        serde_json::to_vec(&json!({"case":scenario,"normal_cleanup":true})).unwrap(),
    )
    .unwrap();
}
struct Host {
    child: Option<Child>,
    root: PathBuf,
}
impl Host {
    fn logs(&self) -> String {
        ["stage", "stdout", "stderr"]
            .into_iter()
            .map(|name| {
                let mut bytes = Vec::new();
                if let Ok(file) = File::open(self.root.join(name)) {
                    let _ = file.take(64 * 1024).read_to_end(&mut bytes);
                }
                format!("{name}: {}", String::from_utf8_lossy(&bytes))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn finish(mut self, scenario: &str) {
        let deadline = Instant::now() + HOST_BOUND;
        let status = loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "owned host exceeded supervision bound\n{}",
                self.logs()
            );
            thread::sleep(Duration::from_millis(10));
        };
        self.child.take();
        assert!(
            status.success(),
            "owned host failed {status}\n{}",
            self.logs()
        );
        let receipt: Value =
            serde_json::from_slice(&fs::read(self.root.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt, json!({"case":scenario,"normal_cleanup":true}));
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let deadline = Instant::now() + BOUND;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    other => {
                        eprintln!("bounded host reap failed: {other:?}");
                        break;
                    }
                }
            }
        }
    }
}
fn run(scenario: &str) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "archive_feed_host",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("U3_ARCHIVE_FEED_TEMP", root)
        .env("U3_ARCHIVE_FEED_CASE", scenario)
        .env("TMPDIR", root)
        .stdin(Stdio::null())
        .stdout(File::create(root.join("stdout")).unwrap())
        .stderr(File::create(root.join("stderr")).unwrap())
        .spawn()
        .unwrap();
    Host {
        child: Some(child),
        root: root.into(),
    }
    .finish(scenario);
}
#[test]
fn optional_archive_serves_signed_feed_without_autonomous_node_or_index_files() {
    run("healthy");
}
#[test]
fn archive_refresh_keeps_selected_records_and_completion_in_one_publication() {
    run("refresh");
}
#[test]
fn failed_page_traversal_discards_partial_view_and_uses_existing_refused_fallback() {
    run("traversal");
}
#[test]
fn disabled_archive_keeps_old_and_legacy_cache_bytes_across_restart() {
    run("disabled");
}
