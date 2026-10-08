#[path = "support/legacy_avatar.rs"]
mod legacy_avatar;
#[path = "support/node.rs"]
mod node;

use bitcoin::hashes::{sha256, Hash};
use urma_core::{envelope, multipart::VerifiedRecord};
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase};
use wire_app_core::connection::SourceChoice;
use wire_app_engine::{config::Settings, native::service::Service, record::review};

#[test]
fn approved_signed_legacy_plan_refuses_restart_status_and_resume_before_network() {
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
    if let Some(path) = std::env::var_os("WIRE_LEGACY_FIXTURE_OUTPUT") {
        std::fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({"id": id, "plan": plan})).unwrap(),
        )
        .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let signer = Keyring::create(&phrase, "public fixture")
        .unwrap()
        .active()
        .unwrap();
    let mock = node::Mock::new(dir.path(), signer);
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
        source: SourceChoice::LocalNode,
        cache_dir: dir.path().join("runtime"),
    };
    let service = Service::open(dir.path(), settings.clone()).unwrap();
    assert!(service
        .status()
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    assert!(service
        .advance(&id, false)
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    assert!(service
        .advance(&id, true)
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    assert!(service
        .discard_quote(&id)
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    drop(service);
    let restarted = Service::open(dir.path(), settings).unwrap();
    assert!(restarted
        .status()
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    assert!(restarted
        .advance(&id, false)
        .unwrap_err()
        .to_string()
        .contains("exactly 128"));
    assert!(mock.state.lock().unwrap().methods.is_empty());
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    assert_eq!(std::fs::read(directory.join("plan.json")).unwrap(), saved);
    assert_eq!(
        std::fs::read(directory.join("approved")).unwrap(),
        id.as_bytes()
    );
    assert_eq!(
        std::fs::read(dir.path().join("active.json")).unwrap(),
        serde_json::to_vec(&id).unwrap()
    );
    assert!(!directory.join("progress.json").exists());
}
