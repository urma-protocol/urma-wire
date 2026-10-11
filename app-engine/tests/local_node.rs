use bitcoin::{
    absolute, block, consensus::serialize, hashes::Hash, transaction, Amount, Block, CompactTarget,
    ScriptBuf, Transaction, TxIn, TxMerkleNode, TxOut,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use urma_chain::observation::Chain;
use urma_runtime::{
    error::Error,
    node::Node,
    transport::{BlockEncoding, Evidence, Provider},
};
use wire_app_core::connection::SourceChoice;
use wire_app_engine::{
    config::Settings,
    native::{
        local_node::{LocalNode, LocalProvider, LocalSource},
        reader::NodeReader,
    },
};

#[derive(Clone, Copy)]
enum Mode {
    Offline,
    Wrong,
    Syncing,
    Ready,
    Corrupt,
}
struct State {
    mode: Mode,
    auth: String,
    blocks: usize,
    probes: usize,
}
struct Rpc {
    state: Arc<Mutex<State>>,
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
fn blocks() -> Arc<Vec<Block>> {
    let mut previous = Chain::LitecoinTestnet.genesis().unwrap().0;
    Arc::new(
        (1..=20)
            .map(|height| {
                let tx = Transaction {
                    version: transaction::Version::TWO,
                    lock_time: absolute::LockTime::ZERO,
                    input: vec![TxIn {
                        script_sig: ScriptBuf::builder().push_int(height).into_script(),
                        ..TxIn::default()
                    }],
                    output: vec![TxOut {
                        value: Amount::from_sat(1),
                        script_pubkey: ScriptBuf::new(),
                    }],
                };
                let block = Block {
                    header: block::Header {
                        version: block::Version::TWO,
                        prev_blockhash: previous,
                        merkle_root: TxMerkleNode::from_byte_array(
                            tx.compute_txid().to_byte_array(),
                        ),
                        time: 1700000000 + height as u32,
                        bits: CompactTarget::from_consensus(0x1e0ffff0),
                        nonce: height as u32,
                    },
                    txdata: vec![tx],
                };
                previous = block.block_hash();
                block
            })
            .collect(),
    )
}
fn answer(blocks: &[Block], method: &str, args: &[Value]) -> Value {
    match method {
        "getblockhash" if args[0] == 0 => {
            json!(Chain::LitecoinTestnet.genesis().unwrap().0.to_string())
        }
        "getblockhash" => json!(blocks[args[0].as_u64().unwrap() as usize - 1]
            .block_hash()
            .to_string()),
        "getblockchaininfo" => {
            json!({"blocks":blocks.len(),"headers":blocks.len(),"initialblockdownload":false,"bestblockhash":blocks.last().unwrap().block_hash().to_string()})
        }
        "getblock" => json!(hex::encode(serialize(
            blocks
                .iter()
                .find(|b| json!(b.block_hash().to_string()) == args[0])
                .unwrap()
        ))),
        _ => panic!("unexpected method {method}"),
    }
}
impl Rpc {
    fn start(blocks: Arc<Vec<Block>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let state = Arc::new(Mutex::new(State {
            mode: Mode::Offline,
            auth: "Basic Zml4dHVyZTpvbmU=".into(),
            blocks: 0,
            probes: 0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let held = state.clone();
        let stopped = stop.clone();
        let task = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &held, &blocks),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            state,
            address,
            stop,
            thread: Some(task),
        }
    }
    fn settings(&self, dir: &std::path::Path) -> Settings {
        let cookie = dir.join("cookie");
        std::fs::write(&cookie, "fixture:one").unwrap();
        Settings {
            rpc_url: Some(format!("http://{}", self.address)),
            node_auth_file: Some(cookie),
            vault: None,
            unlock_file: None,
            source: SourceChoice::Public,
            cache_dir: dir.join("runtime"),
        }
    }
}
impl Drop for Rpc {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn serve(mut stream: TcpStream, state: &Mutex<State>, blocks: &[Block]) {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut length = 0;
    let mut auth = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        if line == "\r\n" {
            break;
        }
        if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap();
        }
        if line.to_lowercase().starts_with("authorization:") {
            auth = line.split_once(':').unwrap().1.trim().into();
        }
    }
    let mut bytes = vec![0; length];
    if reader.read_exact(&mut bytes).is_err() {
        return;
    }
    let req: Value = serde_json::from_slice(&bytes).unwrap();
    let method = req["method"].as_str().unwrap();
    let mut s = state.lock().unwrap();
    if method == "getblockhash" && req["params"][0] == 0 {
        s.probes += 1;
    }
    let response = if matches!(s.mode, Mode::Offline) || auth != s.auth {
        json!({"result":null,"error":{"code":-28,"message":"fixture unavailable"},"id":req["id"]})
    } else {
        let mut v = answer(blocks, method, req["params"].as_array().unwrap());
        if matches!(s.mode, Mode::Wrong) && method == "getblockhash" {
            v = json!("11".repeat(32));
        }
        if matches!(s.mode, Mode::Syncing) && method == "getblockchaininfo" {
            v["initialblockdownload"] = json!(true);
        }
        if method == "getblock" {
            s.blocks += 1;
            if matches!(s.mode, Mode::Corrupt) {
                v = json!("00");
            }
        }
        json!({"result":v,"error":null,"id":req["id"]})
    };
    drop(s);
    let raw = serde_json::to_vec(&response).unwrap();
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        raw.len()
    );
    let _ = stream.write_all(&raw);
}
struct Public {
    blocks: Arc<Vec<Block>>,
    calls: Arc<AtomicUsize>,
    gate: Arc<(Mutex<bool>, Condvar)>,
}
impl Provider for Public {
    fn label(&self) -> String {
        "fixture public".into()
    }
    fn evidence(&self) -> Evidence {
        Evidence::PublicProviderObservation
    }
    fn block_encoding(&self) -> BlockEncoding {
        BlockEncoding::Core
    }
    fn supports(&self, m: &str) -> bool {
        matches!(m, "getblockhash" | "getblockchaininfo" | "getblock")
    }
    fn call(&self, _: Chain, m: &str, args: &[Value]) -> Result<Value, Error> {
        if m == "getblock" {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let (lock, cv) = &*self.gate;
            let (ready, timeout) = cv
                .wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(12), |x| !*x)
                .unwrap();
            assert!(*ready && !timeout.timed_out(), "fixture gate timeout");
        }
        Ok(answer(&self.blocks, m, args))
    }
}
fn wait(mut yes: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(9);
    while !yes() {
        assert!(Instant::now() < end, "condition timed out");
        thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn active_index_scan_adopts_late_local_source_then_falls_back_and_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let blocks = blocks();
    let rpc = Rpc::start(blocks.clone());
    let settings = rpc.settings(dir.path());
    let LocalSource::Configured(local) = LocalNode::configured(&settings).unwrap() else {
        panic!("configured local source missing")
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let node = Arc::new(
        Node::with_providers(
            Chain::LitecoinTestnet,
            vec![
                Box::new(LocalProvider(local.clone())),
                Box::new(Public {
                    blocks,
                    calls: calls.clone(),
                    gate: gate.clone(),
                }),
            ],
        )
        .unwrap(),
    );
    let held = node.clone();
    let path = dir.path().join("index.json");
    let task = thread::spawn(move || {
        urma_wire::index::sync(&NodeReader(&held), &path, 1, 20, |_, _| {}).unwrap()
    });
    wait(|| calls.load(Ordering::SeqCst) > 0);
    rpc.state.lock().unwrap().mode = Mode::Ready;
    wait(|| local.ready());
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    let (report, index) = task.join().unwrap();
    assert_eq!(report.scanned, 20);
    assert_eq!(index.blocks.last().unwrap().height, 20);
    assert!(rpc.state.lock().unwrap().blocks > 0);
    assert!(calls.load(Ordering::SeqCst) < 20);
    // Same Node/router/reader remains in use across outage, cookie rotation and recovery.
    rpc.state.lock().unwrap().mode = Mode::Offline;
    let before = calls.load(Ordering::SeqCst);
    node.block_at(1).unwrap();
    assert!(calls.load(Ordering::SeqCst) > before);
    assert!(!local.ready());
    {
        let mut s = rpc.state.lock().unwrap();
        s.auth = "Basic Zml4dHVyZTp0d28=".into();
        s.mode = Mode::Ready;
    }
    std::fs::write(settings.node_auth_file.unwrap(), "fixture:two").unwrap();
    wait(|| local.ready());
    let before = calls.load(Ordering::SeqCst);
    node.block_at(2).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), before);
    assert!(!LocalProvider(local.clone()).supports("sendrawtransaction"));
    assert!(!LocalProvider(local.clone()).supports("scantxoutset"));
    assert!(!LocalProvider(local.clone()).supports("getblockchaininfo"));
    assert!(LocalProvider(local.clone())
        .call(Chain::LitecoinTestnet, "sendrawtransaction", &[])
        .is_err());
    // Bad block bytes are validated by the router and fall back, never admitted.
    rpc.state.lock().unwrap().mode = Mode::Corrupt;
    let before = calls.load(Ordering::SeqCst);
    node.block_at(3).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), before + 1);
}
#[test]
fn wrong_genesis_and_initial_download_are_not_eligible() {
    let dir = tempfile::tempdir().unwrap();
    let rpc = Rpc::start(blocks());
    let settings = rpc.settings(dir.path());
    rpc.state.lock().unwrap().mode = Mode::Wrong;
    let LocalSource::Configured(local) = LocalNode::configured(&settings).unwrap() else {
        panic!("configured local source missing")
    };
    wait(|| rpc.state.lock().unwrap().probes > 0);
    assert!(!local.ready());
    let before = rpc.state.lock().unwrap().probes;
    rpc.state.lock().unwrap().mode = Mode::Syncing;
    wait(|| rpc.state.lock().unwrap().probes > before);
    thread::sleep(Duration::from_millis(100));
    assert!(!local.ready());
    rpc.state.lock().unwrap().mode = Mode::Ready;
    wait(|| local.ready());
    local.set_enabled(false);
    assert!(!local.ready());
    assert!(!LocalProvider(local.clone()).supports("getblock"));
}
