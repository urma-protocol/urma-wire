#[path = "support/node.rs"]
mod node;
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase};
use wire_app_core::connection::SourceChoice;
use wire_app_engine::config::Settings;
use wire_app_engine::native::service::Service;

fn settings(rpc_url: Option<String>, cookie: Option<std::path::PathBuf>) -> Settings {
    Settings {
        rpc_url,
        node_auth_file: cookie,
        vault: None,
        unlock_file: None,
        source: SourceChoice::LocalNode,
    }
}

fn refusal(settings: &Settings) -> String {
    match settings.connect() {
        Ok(_node) => panic!("connection must be refused"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn local_node_choice_needs_both_fields_and_a_cookie_file() {
    let dir = tempfile::tempdir().unwrap();
    let half = settings(Some("http://127.0.0.1:1".into()), None);
    assert!(refusal(&half).contains("rpc_url and node_auth_file"));
    let missing = dir.path().join("absent.cookie");
    let no_cookie = settings(Some("http://127.0.0.1:1".into()), Some(missing.clone()));
    let message = refusal(&no_cookie);
    assert!(message.contains("cookie not found"));
    assert!(message.contains(missing.to_str().unwrap()));
    let esplora = half.with_source(SourceChoice::Esplora("https://example.invalid".into()));
    assert!(refusal(&esplora).contains("does not serve"));
}

#[test]
fn adopting_the_local_node_verifies_then_persists_the_choice() {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let keys = Keyring::create(&phrase, "source test fixture").unwrap();
    let mock = node::Mock::new(dir.path(), keys.active().unwrap());
    let app = dir.path().join("app");
    let configured = settings(
        Some(mock.config.rpc_url.clone()),
        Some(mock.config.cookie_file.clone()),
    );
    let mut service = Service::open(&app, configured.with_source(SourceChoice::Public)).unwrap();
    assert_eq!(service.source_status().choice, SourceChoice::Public);
    assert_eq!(service.source_status().endpoints.len(), 3);
    assert!(service.source_status().local_node_configured);
    assert!(service.source_status().cookie_available);
    let adopted = service.adopt_source(SourceChoice::LocalNode).unwrap();
    assert_eq!(adopted.choice, SourceChoice::LocalNode);
    assert_eq!(adopted.endpoints, vec![mock.config.rpc_url.clone()]);
    let stored: SourceChoice =
        serde_json::from_slice(&std::fs::read(app.join("source.json")).unwrap()).unwrap();
    assert_eq!(stored, SourceChoice::LocalNode);
    let refused = service
        .adopt_source(SourceChoice::Esplora("https://example.invalid".into()))
        .unwrap_err();
    assert!(refused.to_string().contains("does not serve"));
    assert_eq!(service.source_status().choice, SourceChoice::LocalNode);
}
