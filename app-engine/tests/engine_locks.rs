#[path = "support/node.rs"]
mod node;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use urma_identity::{keyring::Keyring, vault::UnlockedVault};
use wire_app_core::connection::SourceChoice;
use wire_app_core::feed::SyncPhase;
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
    let keys = Keyring::create(&node::phrase(), "engine lock fixture").unwrap();
    let mock = node::Mock::new(dir.path(), &keys);
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
        archive: None,
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
fn identity_requests_answer_while_a_sync_runs() {
    let harness = harness();
    let sync_started = Instant::now();
    let started = harness.service.start_sync().unwrap();
    assert_eq!(started.phase, SyncPhase::Connecting);
    assert_eq!(started.run, 1);
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
    let mut status = harness.service.sync_snapshot();
    let mut seen = Vec::new();
    while !status.settled() {
        seen.push((status.phase.clone(), status.done, status.total));
        thread::sleep(Duration::from_millis(DELAY_MS / 3));
        status = harness.service.sync_snapshot();
    }
    let slice_elapsed = sync_started.elapsed();
    assert_eq!(status.phase, SyncPhase::Complete);
    assert_eq!(status.indexed_to, Some(100));
    assert!(status.complete_to_tip);
    assert_eq!((status.done, status.total), (6, 6));
    let counted: Vec<u64> = seen
        .iter()
        .filter(|(phase, _done, _total)| *phase == SyncPhase::Verifying)
        .map(|(_phase, done, _total)| *done)
        .collect();
    assert!(counted.windows(2).all(|pair| pair[1] >= pair[0]));
    assert!(
        counted.len() >= 2,
        "progress was observed in steps: {counted:?}"
    );
    assert_eq!(harness.service.start_sync().unwrap().run, 2);
    harness.service.lock();
    let started = Instant::now();
    harness
        .service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let idle_unlock = started.elapsed();
    eprintln!(
        "identity status {status_elapsed:?}, unlock {unlock_elapsed:?} (idle unlock {idle_unlock:?}), 6-block sync {slice_elapsed:?} (mock delay {DELAY_MS} ms per rpc)"
    );
    assert!(slice_elapsed >= Duration::from_millis(DELAY_MS * 4));
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
