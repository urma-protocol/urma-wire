#![cfg(not(target_arch = "wasm32"))]

#[path = "support/node.rs"]
mod node;

use bitcoin::{consensus::deserialize, Transaction};
use std::path::PathBuf;
use urma_core::format::PublicRecord;
use urma_core::multipart::VerifiedRecord;
use urma_identity::keyring::Keyring;
use wire_app_core::{
    connection::SourceChoice,
    model::{Inclusion, Placement, Txid},
};
use wire_app_engine::{config::Settings, native::service::Service};

fn vector(name: &str) -> (Vec<u8>, Transaction) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(name);
    let raw = std::fs::read(path).unwrap();
    let transaction = deserialize(&raw).unwrap();
    (raw, transaction)
}

#[test]
fn mempool_lists_signed_wire_records_until_they_confirm_or_leave() {
    let dir = tempfile::tempdir().unwrap();
    let keys = Keyring::create(&node::phrase(), "public fixture").unwrap();
    let mock = node::Mock::new(dir.path(), &keys);
    let (raw_commit, commit) = vector("proof-post.commit");
    let (raw_reveal, reveal) = vector("proof-post.reveal");
    let verified = VerifiedRecord::verify(reveal.compute_txid(), &reveal, &commit).unwrap();
    let PublicRecord::Post(text) = PublicRecord::decode(verified.record_bytes()).unwrap() else {
        panic!("fixture is not a post")
    };
    let reveal_id = reveal.compute_txid().to_string();
    let commit_id = commit.compute_txid().to_string();
    {
        let mut state = mock.state.lock().unwrap();
        state
            .raw_transactions
            .insert(reveal_id.clone(), hex::encode(&raw_reveal));
        state
            .raw_transactions
            .insert(commit_id.clone(), hex::encode(&raw_commit));
        state.transactions.insert(reveal_id.clone(), false);
        state.transactions.insert(commit_id.clone(), false);
        state.transactions.insert("11".repeat(32), false);
    }
    let settings = Settings {
        rpc_url: Some(mock.config.rpc_url.clone()),
        node_auth_file: Some(mock.config.cookie_file.clone()),
        source: SourceChoice::LocalNode,
        cache_dir: dir.path().join("runtime"),
        ..Settings::default()
    };
    let service = Service::open(&dir.path().join("app"), settings).unwrap();
    let pending = service.mempool().unwrap();
    assert_eq!(pending.len(), 1);
    let post = &pending[0];
    assert_eq!(post.txid, Txid::from_display_hex(&reveal_id).unwrap());
    assert_eq!(post.inclusion, Inclusion::Mempool);
    assert_eq!(post.placement, Placement::Root);
    assert_eq!(post.text, text);
    assert_eq!(post.author.to_hex(), verified.author().to_string());
    let calls = mock.state.lock().unwrap().methods.len();
    assert_eq!(service.mempool().unwrap(), pending);
    let repeated = mock.state.lock().unwrap().methods.len() - calls;
    assert_eq!(repeated, 1, "a listed record is fetched once");
    mock.state
        .lock()
        .unwrap()
        .transactions
        .insert(reveal_id.clone(), true);
    assert!(service.mempool().unwrap().is_empty());
    mock.state
        .lock()
        .unwrap()
        .transactions
        .insert(reveal_id.clone(), false);
    assert_eq!(service.mempool().unwrap(), pending);
    mock.state.lock().unwrap().transactions.remove(&reveal_id);
    assert!(service.mempool().unwrap().is_empty());
}
