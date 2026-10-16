#![cfg(not(target_arch = "wasm32"))]

use serde_json::json;
use std::{net::SocketAddr, path::PathBuf};
use wire_app_core::connection::SourceChoice;
use wire_app_engine::config::{ArchiveSettings, Settings, TokenAccess, NODE_SERVICE_ENDPOINT};

#[test]
fn archive_is_absent_by_default_and_for_existing_or_null_json() {
    assert!(Settings::default().archive.is_none());
    for value in [
        json!({}),
        json!({"rpc_url":"http://127.0.0.1:1234", "node_auth_file":"rpc.cookie"}),
        json!({"archive":null}),
    ] {
        let settings: Settings = serde_json::from_value(value).unwrap();
        assert!(settings.archive.is_none());
    }
}

#[test]
fn explicit_archive_deserializes_ipv4_and_ipv6_without_reading_token_or_dialing() {
    let dir = tempfile::tempdir().unwrap();
    let missing_token = dir.path().join("not-created.token");
    for endpoint in ["127.0.0.1:1234", "[::1]:5678", "127.8.9.10:1234"] {
        let settings: Settings = serde_json::from_value(json!({
            "archive": {"endpoint":endpoint, "token_file":missing_token},
        }))
        .unwrap();
        let archive: ArchiveSettings = settings.archive.unwrap();
        assert_eq!(archive.endpoint, endpoint.parse::<SocketAddr>().unwrap());
        assert_eq!(archive.token_file, missing_token);
        assert!(!missing_token.exists());
    }
}

#[test]
fn nonloopback_endpoint_and_relative_token_file_are_rejected_at_load() {
    for archive in [
        json!({"endpoint":"192.0.2.1:1234", "token_file":"/run/urma/archive.token"}),
        json!({"endpoint":"0.0.0.0:1234", "token_file":"/run/urma/archive.token"}),
        json!({"endpoint":"[2001:db8::1]:1234", "token_file":"/run/urma/archive.token"}),
        json!({"endpoint":"127.0.0.1:1234", "token_file":"archive.token"}),
        json!({"endpoint":"127.0.0.1:1234", "token_file":"./archive.token"}),
    ] {
        let refused = serde_json::from_value::<Settings>(json!({ "archive": archive.clone() }));
        let Err(error) = refused else {
            panic!("accepted {archive}")
        };
        let message = error.to_string();
        assert!(
            message.contains("not loopback") || message.contains("not an absolute path"),
            "{message}"
        );
    }
}

#[test]
fn archive_unknown_fields_and_missing_required_fields_are_rejected() {
    for archive in [
        json!({"endpoint":"127.0.0.1:1234", "token_file":"/run/urma/archive.token", "unknown":true}),
        json!({"endpoint":"127.0.0.1:1234", "token_file":"/run/urma/archive.token", "token":"inline"}),
        json!({"endpoint":"127.0.0.1:1234"}),
        json!({"token_file":"/run/urma/archive.token"}),
        json!({}),
        json!({"endpoint":null, "token_file":"/run/urma/archive.token"}),
        json!({"endpoint":"127.0.0.1:1234", "token_file":null}),
    ] {
        assert!(serde_json::from_value::<Settings>(json!({"archive":archive})).is_err());
    }
}

#[test]
fn archive_endpoint_requires_socket_address_syntax_without_dns() {
    for endpoint in [
        "localhost:1234",
        "archive.example.invalid:1234",
        "http://127.0.0.1:1234",
        "127.0.0.1",
        "127.0.0.1:65536",
        "::1:1234",
        "[::1]",
        "",
    ] {
        assert!(
            serde_json::from_value::<Settings>(json!({
                "archive": {"endpoint":endpoint, "token_file":"/run/urma/archive.token"},
            }))
            .is_err(),
            "accepted {endpoint}"
        );
    }
    for endpoint in [json!(1234), json!([127, 0, 0, 1])] {
        assert!(serde_json::from_value::<Settings>(json!({
            "archive": {"endpoint":endpoint, "token_file":"/run/urma/archive.token"},
        }))
        .is_err());
    }
}

#[test]
fn archive_does_not_select_rpc_or_wallet_configuration() {
    let settings: Settings = serde_json::from_value(json!({
        "archive":{"endpoint":"127.0.0.1:1234", "token_file":"/run/urma/archive.token"},
    }))
    .unwrap();
    assert_eq!(settings.source, SourceChoice::Public);
    assert!(!settings.local_node_configured());
    assert!(settings.rpc_url.is_none());
    assert!(settings.node_auth_file.is_none());
    assert!(settings.vault.is_none());
    assert!(settings.unlock_file.is_none());
}

#[test]
fn changing_chain_source_preserves_archive_rpc_and_wallet_settings() {
    let settings: Settings = serde_json::from_value(json!({
        "rpc_url":"http://127.0.0.1:2345",
        "node_auth_file":"rpc.cookie",
        "vault":"identity.urma",
        "unlock_file":"identity.unlock",
        "archive":{"endpoint":"[::1]:1234", "token_file":"/run/urma/archive.token"},
    }))
    .unwrap();
    for source in [
        SourceChoice::Public,
        SourceChoice::LocalNode,
        SourceChoice::Esplora("https://example.invalid".into()),
    ] {
        let changed = settings.with_source(source.clone());
        assert_eq!(changed.source, source);
        let archive = changed.archive.as_ref().unwrap();
        assert_eq!(
            archive.endpoint,
            settings.archive.as_ref().unwrap().endpoint
        );
        assert_eq!(archive.token_file, PathBuf::from("/run/urma/archive.token"));
        assert_eq!(changed.rpc_url, settings.rpc_url);
        assert_eq!(changed.node_auth_file, settings.node_auth_file);
        assert_eq!(changed.vault, settings.vault);
        assert_eq!(changed.unlock_file, settings.unlock_file);
        assert_eq!(changed.cache_dir, settings.cache_dir);
        assert!(changed.local_node_configured());
    }
}

#[test]
fn discovery_targets_the_installed_node_service_and_local_core_when_nothing_is_configured() {
    let home = PathBuf::from("/home/fixture");
    let mut settings: Settings = serde_json::from_value(json!({})).unwrap();
    settings.discovered(&home);
    let node = settings.archive.as_ref().unwrap();
    assert_eq!(node.endpoint, NODE_SERVICE_ENDPOINT);
    assert_eq!(
        node.endpoint,
        "127.0.0.1:28331".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(node.access, TokenAccess::Service);
    #[cfg(unix)]
    assert_eq!(
        node.token_file,
        PathBuf::from("/var/lib/urma-node/service.token")
    );
    assert_eq!(settings.rpc_url.as_deref(), Some("http://127.0.0.1:19332"));
    assert_eq!(
        settings.node_auth_file,
        Some(home.join(".litecoin/testnet4/.cookie"))
    );
    assert_eq!(
        settings.vault,
        Some(home.join(".local/share/urma/vault.urma"))
    );
    assert!(settings.local_node_configured());
    assert_eq!(settings.source, SourceChoice::Public);
}

#[test]
fn discovery_keeps_configured_node_core_and_vault() {
    let home = PathBuf::from("/home/fixture");
    let mut settings: Settings = serde_json::from_value(json!({
        "rpc_url":"http://127.0.0.1:2345",
        "node_auth_file":"/data/litecoin/.cookie",
        "vault":"/data/vault.urma",
        "archive":{"endpoint":"127.0.0.1:1234", "token_file":"/run/urma/archive.token"},
    }))
    .unwrap();
    settings.discovered(&home);
    let node = settings.archive.as_ref().unwrap();
    assert_eq!(
        node.endpoint,
        "127.0.0.1:1234".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(node.token_file, PathBuf::from("/run/urma/archive.token"));
    assert_eq!(node.access, TokenAccess::Owner);
    assert_eq!(settings.rpc_url.as_deref(), Some("http://127.0.0.1:2345"));
    assert_eq!(
        settings.node_auth_file,
        Some(PathBuf::from("/data/litecoin/.cookie"))
    );
    assert_eq!(settings.vault, Some(PathBuf::from("/data/vault.urma")));
}

#[test]
fn mobile_settings_ignore_a_desktop_config_in_the_owned_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config = json!({
        "rpc_url":"http://127.0.0.1:2345",
        "node_auth_file":"rpc.cookie",
        "archive":{"endpoint":"127.0.0.1:1234", "token_file":"/run/urma/archive.token"},
    });
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let desktop: Settings = serde_json::from_value(config).unwrap();
    assert!(desktop.archive.is_some());
    let mobile = Settings::mobile(dir.path());
    assert!(mobile.archive.is_none());
    assert!(mobile.rpc_url.is_none());
    assert!(mobile.node_auth_file.is_none());
    assert_eq!(mobile.source, SourceChoice::Public);
    assert_eq!(mobile.vault, Some(dir.path().join("identity-v2.urma")));
}
