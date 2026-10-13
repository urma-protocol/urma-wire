#[path = "support/node.rs"]
mod node;
use urma_identity::{keyring::Keyring, vault::UnlockedVault};
use wire_app_core::connection::SourceChoice;
use wire_app_core::publication::{Addressed, Content, Pending};
use wire_app_core::session::Credential;
use wire_app_engine::config::Settings;
use wire_app_engine::native::service::Service;

const PASSWORD: &str = "abandon-fixture-password";

fn post(text: &str) -> Content {
    Content::Post {
        text: text.into(),
        wire: String::new(),
        hashtags: vec![],
        addressed: Addressed::Standalone,
    }
}

fn open(dir: &std::path::Path) -> (node::Mock, Service) {
    let keys = Keyring::create(&node::phrase(), "abandon fixture").unwrap();
    let mock = node::Mock::new(dir, &keys);
    let vault = UnlockedVault::create(keys, PASSWORD, &mut rand::rngs::OsRng).unwrap();
    let vault_path = dir.join("fixture.vault");
    std::fs::write(
        &vault_path,
        vault.seal(&mut rand::rngs::OsRng).unwrap().as_bytes(),
    )
    .unwrap();
    let app = dir.join("app");
    let settings = Settings {
        rpc_url: Some(mock.config.rpc_url.clone()),
        node_auth_file: Some(mock.config.cookie_file.clone()),
        vault: Some(vault_path),
        unlock_file: None,
        source: SourceChoice::LocalNode,
        cache_dir: app.join("runtime"),
    };
    let service = Service::open(&app, settings).unwrap();
    service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    (mock, service)
}

#[test]
fn an_approved_plan_that_never_reached_the_network_can_be_abandoned() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, service) = open(dir.path());
    let quote = service.quote(&post("never sent")).unwrap();
    assert!(!quote.abandonable);
    mock.state.lock().unwrap().send_rejection = Some("mempool full".into());
    let paused = service.advance(&quote.id, true).unwrap_err();
    assert!(paused.to_string().contains("paused"));
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    let Pending::Awaiting(standing) = service.status().unwrap() else {
        panic!("approved plan must still be pending");
    };
    assert!(standing.approved);
    assert_eq!((standing.mempool, standing.confirmed), (0, 0));
    assert!(standing.abandonable);
    assert!(service
        .quote(&post("another"))
        .unwrap_err()
        .to_string()
        .contains("abandon"));
    service.discard_quote(&quote.id).unwrap();
    assert!(matches!(service.status().unwrap(), Pending::Settled));
    mock.state.lock().unwrap().send_rejection = None;
    let fresh = service.quote(&post("another")).unwrap();
    assert!(!fresh.approved);
}

#[test]
fn a_plan_with_a_submitted_commit_must_resume() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, service) = open(dir.path());
    let quote = service.quote(&post("sent")).unwrap();
    let first = service.advance(&quote.id, true).unwrap();
    assert_eq!(first.mempool, 2);
    assert!(!first.abandonable);
    assert_eq!(mock.state.lock().unwrap().submissions.len(), 2);
    assert!(service.discard_quote(&quote.id).is_err());
    let Pending::Awaiting(standing) = service.status().unwrap() else {
        panic!("submitted plan must still be pending");
    };
    assert!(!standing.abandonable);
}
