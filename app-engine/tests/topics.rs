use bitcoin::{
    absolute,
    consensus::{deserialize, serialize},
    hashes::Hash,
    secp256k1::{Keypair, Secp256k1, SecretKey},
    transaction::Version,
    Amount, Block, BlockHash, Network, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness,
};
use std::{collections::HashMap, num::NonZeroU64};
use urma_chain::observation::Chain;
use urma_core::{format::PublicRecord, topics::Topics};
use urma_runtime::publication::prepare_signed_bytes;
use urma_wallet::{
    funding::Funding,
    wallet::{FeeBudget, FeeRate},
};
use urma_wire::{
    index::{self, Index},
    reader::Reader,
};
use wire_app_core::feed::matches_topic;
use wire_app_core::model::Placement;
use wire_app_engine::projection;
use wire_app_engine::record::wire_txid;

struct Fixture {
    block: Block,
    commits: HashMap<Txid, Transaction>,
}
impl Reader for Fixture {
    type Error = std::io::Error;
    fn genesis(&self) -> Result<BlockHash, Self::Error> {
        Ok(self.block.block_hash())
    }
    fn tip_height(&self) -> Result<u64, Self::Error> {
        Ok(0)
    }
    fn block_hash(&self, _: u64) -> Result<BlockHash, Self::Error> {
        Ok(self.block.block_hash())
    }
    fn block(&self, _: u64) -> Result<(Block, BlockHash), Self::Error> {
        Ok((self.block.clone(), self.block.block_hash()))
    }
    fn transaction(&self, id: Txid) -> Result<Transaction, Self::Error> {
        self.commits
            .get(&id)
            .cloned()
            .ok_or(std::io::Error::other("missing fixture commit"))
    }
}

fn pair(record: PublicRecord) -> (Transaction, Transaction) {
    let signer =
        Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7; 32]).unwrap());
    let funding = Transaction {
        version: Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn::default()],
        output: vec![TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: urma_wallet::signing::script(&signer).unwrap(),
        }],
    };
    let prepared = prepare_signed_bytes(
        &record.encode().unwrap(),
        &signer,
        Funding {
            raw_transaction: hex::encode(serialize(&funding)),
            vout: 0,
        },
        Chain::BitcoinRegtest,
        FeeBudget {
            chain: Chain::BitcoinRegtest.genesis().unwrap(),
            rate: FeeRate(NonZeroU64::new(1).unwrap()),
            maximum_base_units: 10_000,
        },
    )
    .unwrap();
    (
        deserialize(&hex::decode(prepared.plan.commit).unwrap()).unwrap(),
        deserialize(&hex::decode(prepared.plan.reveal).unwrap()).unwrap(),
    )
}

#[test]
fn verified_ingestion_migration_projection_and_cross_wire_filtering() {
    let legacy = pair(PublicRecord::Post(
        "#investigation /rosint are literal legacy text".into(),
    ));
    let target = legacy.1.compute_txid();
    let structured = pair(PublicRecord::WirePost {
        topics: Topics {
            wire: "rosint".into(),
            hashtags: vec!["investigation".into()],
        },
        text: "post".into(),
    });
    let reply = pair(PublicRecord::WireReply {
        target,
        topics: Topics {
            wire: "other".into(),
            hashtags: vec!["investigation".into()],
        },
        text: "reply".into(),
    });
    let mut block = bitcoin::blockdata::constants::genesis_block(Network::Regtest);
    let mut commits = HashMap::new();
    for (commit, reveal) in [legacy, structured, reply] {
        commits.insert(commit.compute_txid(), commit);
        block.txdata.push(reveal);
    }
    block.txdata[0].input[0].witness = Witness::from_slice(&[vec![0; 32]]);
    let commitment = Block::compute_witness_commitment(&block.witness_root().unwrap(), &[0; 32]);
    block.txdata[0].output.push(TxOut {
        value: Amount::ZERO,
        script_pubkey: ScriptBuf::from_bytes(
            [
                vec![0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed],
                commitment.to_byte_array().to_vec(),
            ]
            .concat(),
        ),
    });
    block.header.merkle_root = block.compute_merkle_root().unwrap();
    let fixture = Fixture { block, commits };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");
    let (report, _index) = index::sync(&fixture, &path, 0, 1, |_height, _tip| ()).unwrap();
    assert_eq!(report.records, 3);
    assert!(report.complete_to_tip);
    let mut index = Index::load(&path).unwrap();
    assert_eq!(index.format, "URMA-WIRE-INDEX-2");
    let feed = projection::project(&index, true).unwrap();
    assert_eq!(feed.posts[1].wire, "rosint");
    assert_eq!(feed.posts[1].hashtags, vec!["investigation".to_owned()]);
    assert_eq!(feed.posts[0].placement, Placement::Reply(wire_txid(target)));
    assert_eq!(
        feed.posts
            .iter()
            .filter(|post| matches_topic(post, "/rosint"))
            .count(),
        1
    );
    assert_eq!(
        feed.posts
            .iter()
            .filter(|post| matches_topic(post, "#investigation"))
            .count(),
        2
    );
    assert_eq!(
        feed.posts
            .iter()
            .filter(|post| matches_topic(post, "/other"))
            .count(),
        1
    );
    assert!(!matches_topic(&feed.posts[2], "#investigation"));
    assert_eq!(feed.posts[2].reply_count, 1);
    let replies = projection::replies(&index, wire_txid(target)).unwrap();
    assert_eq!(replies[0].wire, "other");
    let rendered = urma_wire::view::records(&index, 10).unwrap();
    assert_eq!(rendered[0]["payload"]["hashtags"][0], "investigation");
    index.format = "URMA-WIRE-INDEX-1".into();
    index
        .entries
        .retain(|entry| entry.txid == target.to_string());
    index.persist(&path).unwrap();
    let (migrated, _migrated_index) =
        index::sync(&fixture, &path, 0, 1, |_height, _tip| ()).unwrap();
    assert_eq!((migrated.scanned, migrated.records), (1, 3));
    assert_eq!(Index::load(&path).unwrap().format, "URMA-WIRE-INDEX-2");
    assert_eq!(
        index::sync(&fixture, &path, 0, 1, |_height, _tip| ())
            .unwrap()
            .0
            .scanned,
        0
    );
}
