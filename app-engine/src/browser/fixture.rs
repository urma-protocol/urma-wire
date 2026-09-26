use crate::browser::dispatch::handle;
use crate::browser::session::{with_signer, Epoch};
use crate::browser::source::Source;
use crate::config::NETWORK;
use crate::errors::BrowserError;
use bitcoin::absolute::LockTime;
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::hashes::Hash;
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, Block, BlockHash, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use serde::Deserialize;
use serde_json::{json, Value};
use urma_identity::identity::IdentitySigner;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wire_app_core::contract::Request;

#[derive(Deserialize)]
struct Probe {
    url: String,
}

#[derive(Deserialize)]
struct Mined {
    previous: String,
    transactions: Vec<String>,
    nonce: u32,
}

#[derive(Deserialize)]
struct Raw {
    raw: String,
}

#[wasm_bindgen]
pub async fn browser_fixture_command(cmd: String, args: String) -> Result<String, JsValue> {
    match run(&cmd, &args).await {
        Ok(answer) => Ok(answer),
        Err(error) => Err(JsValue::from_str(&error.to_string())),
    }
}

async fn run(cmd: &str, args: &str) -> Result<String, BrowserError> {
    match cmd {
        "engine" => {
            let request: Request = serde_json::from_str(args)?;
            Ok(serde_json::to_string(&handle(request).await?)?)
        }
        "fixture-sign" => signed(),
        "fixture-source-probe" => Ok(probe(serde_json::from_str(args)?).await?.to_string()),
        "fixture-chain" => Ok(chain()?.to_string()),
        "fixture-block" => Ok(block(serde_json::from_str(args)?)?.to_string()),
        "fixture-txid" => Ok(txid(serde_json::from_str(args)?)?.to_string()),
        _unknown => Err(BrowserError::FixtureCommand),
    }
}

fn signed() -> Result<String, BrowserError> {
    with_signer(Epoch::Any, |key| {
        let message = bitcoin::secp256k1::Message::from_digest([42; 32]);
        let signature = key.sign_author(message)?;
        bitcoin::secp256k1::Secp256k1::new().verify_schnorr(
            &signature,
            &message,
            &key.author().0,
        )?;
        Ok(signature.to_string())
    })
}

async fn probe(asked: Probe) -> Result<Value, BrowserError> {
    let source = Source::connect(asked.url).await?;
    let (height, _anchor) = source.tip().await?;
    let block = source.block(height).await?;
    Ok(json!({"height": height, "hash": block.block_hash().to_string(), "transactions": block.txdata.len()}))
}

fn mined(previous: BlockHash, transactions: Vec<Transaction>, nonce: u32) -> Result<Block, BrowserError> {
    let mut block = bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Regtest);
    block.header.prev_blockhash = previous;
    block.header.nonce = nonce;
    block.txdata.extend(transactions);
    block.txdata[0].input[0].witness = Witness::from_slice(&[vec![0; 32]]);
    let Some(root) = block.witness_root() else {
        return Err(BrowserError::Fixture("witness root"));
    };
    let commitment = Block::compute_witness_commitment(&root, &[0; 32]);
    let mut bytes = vec![0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed];
    bytes.extend_from_slice(commitment.as_byte_array());
    block.txdata[0].output.push(TxOut {
        value: Amount::ZERO,
        script_pubkey: ScriptBuf::from_bytes(bytes),
    });
    let Some(merkle) = block.compute_merkle_root() else {
        return Err(BrowserError::Fixture("merkle root"));
    };
    block.header.merkle_root = merkle;
    Ok(block)
}

fn encoded(block: &Block) -> Value {
    json!({"hash": block.block_hash().to_string(), "raw": hex::encode(serialize(block))})
}

fn funding() -> Result<Transaction, BrowserError> {
    with_signer(Epoch::Any, |signer| {
        Ok(Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: bitcoin::Txid::all_zeros(),
                    vout: 1,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(100_000_000),
                script_pubkey: urma_wallet::signing::script(signer)?,
            }],
        })
    })
}

fn chain() -> Result<Value, BrowserError> {
    let funding = funding()?;
    let genesis = NETWORK.genesis()?.0;
    let mut previous = genesis;
    let mut blocks = Vec::new();
    for height in 1..=200u32 {
        let transactions = match height {
            1 => vec![funding.clone()],
            _later => vec![],
        };
        let block = mined(previous, transactions, height)?;
        previous = block.block_hash();
        blocks.push(encoded(&block));
    }
    Ok(json!({
        "genesis": genesis.to_string(),
        "blocks": blocks,
        "funding": {"txid": funding.compute_txid().to_string(), "raw": hex::encode(serialize(&funding)), "value": 100_000_000},
    }))
}

fn block(asked: Mined) -> Result<Value, BrowserError> {
    let mut transactions = Vec::new();
    for raw in &asked.transactions {
        transactions.push(deserialize::<Transaction>(&hex::decode(raw)?)?);
    }
    Ok(encoded(&mined(asked.previous.parse()?, transactions, asked.nonce)?))
}

fn txid(asked: Raw) -> Result<Value, BrowserError> {
    let tx: Transaction = deserialize(&hex::decode(asked.raw)?)?;
    Ok(json!(tx.compute_txid().to_string()))
}
