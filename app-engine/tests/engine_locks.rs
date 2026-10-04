#[path = "support/node.rs"]
mod node;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_core::connection::SourceChoice;
use wire_app_core::session::{Credential, Session};
use wire_app_engine::config::Settings;
use wire_app_engine::errors::NativeError;
use wire_app_engine::native::service::Service;

const PASSWORD: &str = "engine-lock-fixture";
const DELAY_MS: u64 = 150;

struct Harness {
    _dir: tempfile::TempDir,
    mock: node::Mock,
    service: Arc<Service>,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let keys = Keyring::create(&phrase, "engine lock fixture").unwrap();
    let mock = node::Mock::new(dir.path(), keys.active().unwrap());
    mock.state.lock().unwrap().delay_ms = DELAY_MS;
    let vault = UnlockedVault::create(keys, PASSWORD, &mut rand::rngs::OsRng).unwrap();
    let vault_path = dir.path().join("fixture.vault");
    std::fs::write(
        &vault_path,
        vault.seal(&mut rand::rngs::OsRng).unwrap().as_bytes(),
    )
    .unwrap();
    let app = dir.path().join("app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("ltc-testnet-start.json"), "95").unwrap();
    let settings = Settings {
        rpc_url: Some(mock.config.rpc_url.clone()),
        node_auth_file: Some(mock.config.cookie_file.clone()),
        vault: Some(vault_path),
        unlock_file: None,
        source: SourceChoice::LocalNode,
        cache_dir: app.join("runtime"),
    };
    let service = Arc::new(Service::open(&app, settings).unwrap());
    Harness {
        _dir: dir,
        mock,
        service,
    }
}

#[test]
fn identity_requests_answer_while_a_sync_slice_runs() {
    let harness = harness();
    let service = harness.service.clone();
    let slice = thread::spawn(move || {
        let started = Instant::now();
        let progress = service.sync_slice(5).unwrap();
        (progress, started.elapsed())
    });
    thread::sleep(Duration::from_millis(DELAY_MS * 2));
    let started = Instant::now();
    let status = harness.service.identity_status().unwrap();
    let status_elapsed = started.elapsed();
    assert_eq!(status.session, Session::Locked);
    let started = Instant::now();
    harness
        .service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let unlock_elapsed = started.elapsed();
    assert!(matches!(
        harness.service.identity_status().unwrap().session,
        Session::Unlocked(_)
    ));
    let (progress, slice_elapsed) = slice.join().unwrap();
    assert_eq!(progress.scanned, 5);
    assert_eq!(progress.indexed_to, Some(99));
    harness.service.lock();
    let started = Instant::now();
    harness
        .service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let idle_unlock = started.elapsed();
    eprintln!(
        "identity status {status_elapsed:?}, unlock {unlock_elapsed:?} (idle unlock {idle_unlock:?}), 5-block slice {slice_elapsed:?} (mock delay {DELAY_MS} ms per rpc)"
    );
    assert!(slice_elapsed >= Duration::from_millis(DELAY_MS * 10));
    assert!(status_elapsed < Duration::from_millis(DELAY_MS));
    assert!(unlock_elapsed < idle_unlock * 2 + Duration::from_millis(DELAY_MS));
    assert!(harness
        .mock
        .state
        .lock()
        .unwrap()
        .methods
        .contains(&"getblock".to_owned()));
}

#[test]
fn a_wallet_query_interrupted_by_lock_is_discarded() {
    let harness = harness();
    harness
        .service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let service = harness.service.clone();
    let query = thread::spawn(move || service.wallet());
    thread::sleep(Duration::from_millis(DELAY_MS));
    harness.service.lock();
    let outcome = query.join().unwrap();
    assert!(matches!(outcome, Err(NativeError::Locked)));
    assert_eq!(
        harness.service.identity_status().unwrap().session,
        Session::Locked
    );
    harness
        .service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let summary = harness.service.wallet().unwrap();
    assert_eq!(summary.utxo_count, 1);
}
