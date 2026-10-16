#[path = "support/legacy_avatar.rs"]
mod legacy_avatar;
#[path = "support/node.rs"]
mod node;

use bitcoin::hashes::{sha256, Hash};
use urma_core::{envelope, multipart::VerifiedRecord};
use urma_identity::keyring::Keyring;
use wire_app_core::{connection::SourceChoice, publication::Pending};
use wire_app_engine::{config::Settings, native::service::Service, record::review};

#[test]
fn approved_signed_legacy_plan_is_dropped_before_network() {
    let plan = legacy_avatar::signed_plan();
    let commit =
        bitcoin::consensus::deserialize(&hex::decode(&plan.records[0].commit).unwrap()).unwrap();
    let reveal: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&hex::decode(&plan.records[0].reveal).unwrap()).unwrap();
    let proof = VerifiedRecord::verify(reveal.compute_txid(), &reveal, &commit).unwrap();
    assert_eq!(proof.record_bytes().len(), 520);
    assert!(review(proof.record_bytes()).is_err());
    assert!(envelope::verify_reveal(&reveal, &commit).is_err());
    // This is also the offline plan validation called by browser Active::validate.
    assert!(plan
        .validate()
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    let saved = serde_json::to_vec(&plan).unwrap();
    let id = sha256::Hash::hash(&saved).to_string();
    let dir = tempfile::tempdir().unwrap();
    let keys = Keyring::create(&node::phrase(), "public fixture").unwrap();
    let mock = node::Mock::new(dir.path(), &keys);
    let directory = dir.path().join("plans").join(&id);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("plan.json"), &saved).unwrap();
    std::fs::write(directory.join("approved"), id.as_bytes()).unwrap();
    std::fs::write(
        dir.path().join("active.json"),
        serde_json::to_vec(&id).unwrap(),
    )
    .unwrap();
    let settings = Settings {
        rpc_url: Some(mock.config.rpc_url.clone()),
        node_auth_file: Some(mock.config.cookie_file.clone()),
        vault: None,
        unlock_file: None,
        archive: None,
        source: SourceChoice::LocalNode,
        cache_dir: dir.path().join("runtime"),
    };
    let service = Service::open(dir.path(), settings.clone()).unwrap();
    assert!(matches!(service.status().unwrap(), Pending::Settled));
    assert!(!dir.path().join("active.json").exists());
    assert!(!directory.exists());
    assert!(service
        .advance(&id, true)
        .unwrap_err()
        .to_string()
        .contains("No prepared publication"));
    service.discard_quote(&id).unwrap();
    drop(service);
    let restarted = Service::open(dir.path(), settings).unwrap();
    assert!(matches!(restarted.status().unwrap(), Pending::Settled));
    assert!(mock.state.lock().unwrap().methods.is_empty());
    assert!(mock.state.lock().unwrap().submissions.is_empty());
}

#[test]
fn unreadable_saved_plans_are_dropped_instead_of_blocking_publication() {
    let dir = tempfile::tempdir().unwrap();
    let id = "4da09952943d5a9169a5624ca202cbd5ab4e69d6351e88abc9f92f55d47192d0";
    let directory = dir.path().join("plans").join(id);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("plan.json"),
        br#"{"version":1,"records":[{"funding":{"vout":1}}]}"#,
    )
    .unwrap();
    std::fs::write(directory.join("approved"), id.as_bytes()).unwrap();
    let active = dir.path().join("active.json");
    std::fs::write(&active, serde_json::to_vec(id).unwrap()).unwrap();
    let elsewhere = dir.path().join("plans").join("kept");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let service = Service::open(dir.path(), Settings::default()).unwrap();
    assert!(matches!(service.status().unwrap(), Pending::Settled));
    assert!(!active.exists());
    assert!(!directory.exists());
    assert!(elsewhere.exists());
    for pointer in [&b"\"../../outside\""[..], b"not json"] {
        std::fs::write(&active, pointer).unwrap();
        assert!(matches!(service.status().unwrap(), Pending::Settled));
        assert!(!active.exists());
        assert!(dir.path().join("plans").exists());
    }
}
