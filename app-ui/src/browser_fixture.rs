//! Public synthetic chain fixture helpers, absent from normal builds.
use super::{
    browser::with_signer,
    browser_source::{error, CHAIN},
};
use bitcoin::{
    consensus::{deserialize, serialize},
    hashes::Hash,
    Block, Transaction,
};
use serde_json::{json, Value};

fn block(previous: bitcoin::BlockHash, transactions: Vec<Transaction>, nonce: u32) -> Block {
    let mut block = bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Regtest);
    block.header.prev_blockhash = previous;
    block.header.nonce = nonce;
    block.txdata.extend(transactions);
    block.txdata[0].input[0].witness = bitcoin::Witness::from_slice(&[vec![0; 32]]);
    let commitment = Block::compute_witness_commitment(&block.witness_root().unwrap(), &[0; 32]);
    let mut bytes = vec![0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed];
    bytes.extend_from_slice(commitment.as_byte_array());
    block.txdata[0].output.push(bitcoin::TxOut {
        value: bitcoin::Amount::ZERO,
        script_pubkey: bitcoin::ScriptBuf::from_bytes(bytes),
    });
    block.header.merkle_root = block.compute_merkle_root().unwrap();
    block
}
fn encode(block: &Block) -> Value {
    json!({"hash": block.block_hash().to_string(), "raw": hex::encode(serialize(block))})
}
pub fn command(cmd: &str, args: &Value) -> Result<Value, String> {
    match cmd {
        "fixture-chain" => {
            let funding = with_signer(None, |signer| {
                Ok(Transaction {
                    version: bitcoin::transaction::Version::TWO,
                    lock_time: bitcoin::absolute::LockTime::ZERO,
                    input: vec![bitcoin::TxIn {
                        previous_output: bitcoin::OutPoint {
                            txid: bitcoin::Txid::all_zeros(),
                            vout: 1,
                        },
                        ..Default::default()
                    }],
                    output: vec![bitcoin::TxOut {
                        value: bitcoin::Amount::from_sat(100_000_000),
                        script_pubkey: urma_wallet::signing::script(signer).map_err(error)?,
                    }],
                })
            })?;
            let mut previous = CHAIN.genesis().map_err(error)?.0;
            let mut blocks = Vec::new();
            for height in 1..=200 {
                let block = block(
                    previous,
                    if height == 1 {
                        vec![funding.clone()]
                    } else {
                        vec![]
                    },
                    height,
                );
                previous = block.block_hash();
                blocks.push(encode(&block));
            }
            Ok(
                json!({"genesis": CHAIN.genesis().map_err(error)?.0.to_string(), "blocks": blocks, "funding": {"txid": funding.compute_txid().to_string(), "raw": hex::encode(serialize(&funding)), "value": 100_000_000}}),
            )
        }
        "fixture-block" => {
            let previous = args["previous"]
                .as_str()
                .ok_or("previous")?
                .parse()
                .map_err(error)?;
            let txs: Vec<String> =
                serde_json::from_value(args["transactions"].clone()).map_err(error)?;
            let txs: Result<Vec<Transaction>, String> = txs
                .iter()
                .map(|raw| deserialize(&hex::decode(raw).map_err(error)?).map_err(error))
                .collect();
            Ok(encode(&block(
                previous,
                txs?,
                args["nonce"].as_u64().unwrap_or(201) as u32,
            )))
        }
        "fixture-txid" => {
            let tx: Transaction =
                deserialize(&hex::decode(args["raw"].as_str().ok_or("raw")?).map_err(error)?)
                    .map_err(error)?;
            Ok(json!(tx.compute_txid().to_string()))
        }
        _ => Err("Unknown chain fixture command".into()),
    }
}
