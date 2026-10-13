use bitcoin::{
    absolute,
    block::{Header, Version as BlockVersion},
    consensus::{deserialize, serialize},
    hashes::Hash,
    transaction::Version,
    Amount, Block, CompactTarget, ScriptBuf, Transaction, TxIn, TxMerkleNode, TxOut,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use urma_chain::observation::Chain;
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase};
use urma_runtime::node::NodeConfig;
use urma_wallet::account::Account;

pub fn phrase() -> IdentityPhrase {
    IdentityPhrase::parse(&format!("{}about", "abandon ".repeat(11))).unwrap()
}

pub fn funder(keys: &Keyring) -> Account {
    Account::from_root(Chain::LitecoinTestnet, keys.root()).unwrap()
}

#[derive(Default)]
pub struct State {
    pub transactions: HashMap<String, bool>,
    pub raw_transactions: HashMap<String, String>,
    pub submissions: Vec<String>,
    pub methods: Vec<String>,
    pub preflight: Option<String>,
    pub send_rejection: Option<String>,
    pub send_response_lost: bool,
    pub unavailable: bool,
    pub auto_confirm: bool,
    pub reorg: bool,
    pub next_block: bool,
    pub delay_ms: u64,
}

pub const CHAIN_BLOCKS: u64 = 101;

fn chain_blocks(funding: &Transaction) -> Vec<Block> {
    let mut previous = Chain::LitecoinTestnet.genesis().unwrap().0;
    let mut blocks = Vec::new();
    for height in 1..=CHAIN_BLOCKS {
        let coinbase = Transaction {
            version: Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                script_sig: ScriptBuf::builder()
                    .push_int(i64::try_from(height).unwrap())
                    .into_script(),
                ..TxIn::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(1_000_000),
                script_pubkey: funding.output[0].script_pubkey.clone(),
            }],
        };
        let header = Header {
            version: BlockVersion::TWO,
            prev_blockhash: previous,
            merkle_root: TxMerkleNode::from_byte_array(coinbase.compute_txid().to_byte_array()),
            time: 1_700_000_000 + u32::try_from(height).unwrap(),
            bits: CompactTarget::from_consensus(0x1e0ffff0),
            nonce: u32::try_from(height).unwrap(),
        };
        let block = Block {
            header,
            txdata: vec![coinbase],
        };
        previous = block.block_hash();
        blocks.push(block);
    }
    blocks
}

pub struct Mock {
    pub state: Arc<Mutex<State>>,
    pub config: NodeConfig,
    stopped: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Mock {
    pub fn new(directory: &Path, keys: &Keyring) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let funding = Transaction {
            version: Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn::default()],
            output: vec![TxOut {
                value: Amount::from_sat(100_000_000),
                script_pubkey: funder(keys).receive_script().unwrap(),
            }],
        };
        let state = Arc::new(Mutex::new(State::default()));
        let stopped = Arc::new(AtomicBool::new(false));
        let (worker_state, worker_stop, worker_funding) =
            (state.clone(), stopped.clone(), funding.clone());
        let blocks = chain_blocks(&funding);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut bytes = vec![0; length];
                reader.read_exact(&mut bytes).unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                let mut state = worker_state.lock().unwrap();
                let response = match respond(&mut state, &worker_funding, &blocks, &request) {
                    Ok(value) => json!({"result":value,"error":null,"id":request["id"]}),
                    Err((code, reason)) => {
                        json!({"result":null,"error":{"code":code,"message":reason},"id":request["id"]})
                    }
                };
                let delay = state.delay_ms;
                drop(state);
                if delay > 0 {
                    thread::sleep(Duration::from_millis(delay));
                }
                let body = serde_json::to_vec(&response).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let cookie = directory.join("mock.cookie");
        std::fs::write(&cookie, "public-fixture:public-fixture").unwrap();
        Self {
            state,
            config: NodeConfig {
                chain: Chain::LitecoinTestnet,
                rpc_url: format!("http://{address}"),
                cookie_file: cookie,
            },
            stopped,
            thread: Some(thread),
        }
    }

    pub fn confirm_all(&self) {
        for confirmed in self.state.lock().unwrap().transactions.values_mut() {
            *confirmed = true;
        }
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let address = self.config.rpc_url.strip_prefix("http://").unwrap();
        let _connection = TcpStream::connect(address).unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}

pub fn txid(raw: &str) -> String {
    deserialize::<Transaction>(&hex::decode(raw).unwrap())
        .unwrap()
        .compute_txid()
        .to_string()
}

fn respond(
    state: &mut State,
    funding: &Transaction,
    blocks: &[Block],
    request: &Value,
) -> Result<Value, (i64, String)> {
    let method = request["method"].as_str().unwrap();
    state.methods.push(method.into());
    let args = &request["params"];
    let tip = if state.next_block { 101 } else { 100 };
    let hash = blocks[tip - 1].block_hash().to_string();
    match method {
        "getblockhash" => Ok(if args[0] == 0 {
            json!(Chain::LitecoinTestnet.genesis().unwrap().0.to_string())
        } else if state.reorg {
            json!("22".repeat(32))
        } else {
            let height = usize::try_from(args[0].as_u64().unwrap()).unwrap();
            json!(blocks[height - 1].block_hash().to_string())
        }),
        "getblockchaininfo" => Ok(json!({"blocks":tip,"bestblockhash":hash})),
        "getblockheader" => Ok(json!({"height":tip})),
        "getblock" => {
            let wanted = args[0].as_str().unwrap();
            let block = blocks
                .iter()
                .find(|block| block.block_hash().to_string() == wanted)
                .expect("fixture block");
            Ok(json!(hex::encode(serialize(block))))
        }
        "getindexinfo" => Ok(json!({"txindex":{"synced":true}})),
        "scantxoutset" => Ok(
            json!({"success":true,"unspents":[{"txid":funding.compute_txid(),"vout":0,"amount":1,"height":1,"scriptPubKey":hex::encode(funding.output[0].script_pubkey.as_bytes())}]}),
        ),
        "gettxout" => Ok(
            json!({"value":1,"confirmations":100,"scriptPubKey":{"hex":hex::encode(funding.output[0].script_pubkey.as_bytes())}}),
        ),
        "getrawmempool" => Ok(json!(state
            .transactions
            .iter()
            .filter(|(_, confirmed)| !**confirmed)
            .map(|(id, _)| id)
            .collect::<Vec<_>>())),
        "getrawtransaction" if args[0] == json!(funding.compute_txid()) => {
            Ok(json!(hex::encode(serialize(funding))))
        }
        "getrawtransaction" => {
            if state.unavailable {
                return Err((-28, "fixture source unavailable".into()));
            }
            if args[1] == false {
                return state
                    .raw_transactions
                    .get(args[0].as_str().unwrap())
                    .map(|raw| json!(raw))
                    .ok_or((-5, "fixture raw transaction missing".into()));
            }
            match state.transactions.get(args[0].as_str().unwrap()) {
                Some(true) => Ok(json!({"confirmations":1,"blockhash":hash})),
                Some(false) => Ok(json!({"confirmations":0})),
                None => Err((-5, "fixture missing".into())),
            }
        }
        "testmempoolaccept" => {
            let id = txid(args[0][0].as_str().unwrap());
            Ok(match &state.preflight {
                Some(reason) => json!([{"txid":id,"allowed":false,"reject-reason":reason}]),
                None => json!([{"txid":id,"allowed":true}]),
            })
        }
        "sendrawtransaction" => {
            if let Some(reason) = &state.send_rejection {
                return Err((-26, reason.clone()));
            }
            let raw = args[0].as_str().unwrap();
            let id = txid(raw);
            state.submissions.push(raw.into());
            state.transactions.insert(id.clone(), state.auto_confirm);
            if state.send_response_lost {
                state.unavailable = true;
                return Err((-28, "fixture response lost after acceptance".into()));
            }
            Ok(json!(id))
        }
        other => panic!("unexpected RPC {other}"),
    }
}
