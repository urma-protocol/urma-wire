#![cfg(not(target_arch = "wasm32"))]

use serde_json::json;
use std::{net::SocketAddr, path::PathBuf};
use wire_app_core::connection::SourceChoice;
use wire_app_engine::config::{ArchiveSettings, Settings};

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
    // Loopback authorization belongs to Client::new, not config parsing.
    for endpoint in ["127.0.0.1:1234", "[::1]:5678", "192.0.2.1:1234"] {
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
fn archive_unknown_fields_and_missing_required_fields_are_rejected() {
    for archive in [
        json!({"endpoint":"127.0.0.1:1234", "token_file":"archive.token", "unknown":true}),
        json!({"endpoint":"127.0.0.1:1234", "token_file":"archive.token", "token":"inline"}),
        json!({"endpoint":"127.0.0.1:1234"}),
        json!({"token_file":"archive.token"}),
        json!({}),
        json!({"endpoint":null, "token_file":"archive.token"}),
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
                "archive": {"endpoint":endpoint, "token_file":"archive.token"},
            }))
            .is_err(),
            "accepted {endpoint}"
        );
    }
    for endpoint in [json!(1234), json!([127, 0, 0, 1])] {
        assert!(serde_json::from_value::<Settings>(json!({
            "archive": {"endpoint":endpoint, "token_file":"archive.token"},
        }))
        .is_err());
    }
}

#[test]
fn archive_does_not_select_rpc_or_wallet_configuration() {
    let settings: Settings = serde_json::from_value(json!({
        "archive":{"endpoint":"127.0.0.1:1234", "token_file":"archive.token"},
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
        "archive":{"endpoint":"[::1]:1234", "token_file":"archive.token"},
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
        assert_eq!(archive.token_file, PathBuf::from("archive.token"));
        assert_eq!(changed.rpc_url, settings.rpc_url);
        assert_eq!(changed.node_auth_file, settings.node_auth_file);
        assert_eq!(changed.vault, settings.vault);
        assert_eq!(changed.unlock_file, settings.unlock_file);
        assert_eq!(changed.cache_dir, settings.cache_dir);
        assert!(changed.local_node_configured());
    }
}

#[test]
fn mobile_settings_ignore_a_desktop_config_in_the_owned_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config = json!({
        "rpc_url":"http://127.0.0.1:2345",
        "node_auth_file":"rpc.cookie",
        "archive":{"endpoint":"127.0.0.1:1234", "token_file":"archive.token"},
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
    assert_eq!(mobile.vault, Some(dir.path().join("identity.urma")));
}
