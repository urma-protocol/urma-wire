use wire_app_core::connection::{safe_endpoint, ConnectionHealth, Trouble};

#[test]
fn check_classifies_current_catchup_and_failure() {
    let mut health = ConnectionHealth {
        network: "Litecoin Testnet".into(),
        source_type: "Public source".into(),
        endpoint: "https://example.org".into(),
        checked_at_ms: 1,
        remote_height: Some(100),
        local_start: Some(90),
        local_height: Some(100),
        trouble: Trouble::Clear,
    };
    assert_eq!(health.status(), "Up to date at last check");
    health.local_height = Some(99);
    assert_eq!(health.status(), "Catch-up needed");
    health.local_height = Some(101);
    assert_eq!(health.status(), "Local index ahead of checked source");
    health.trouble = Trouble::SourceUnverified("Source unavailable".into());
    assert_eq!(health.status(), "Source unavailable");
    health.trouble = Trouble::IndexUnreadable("Local index unavailable".into());
    assert_eq!(health.status(), "Local index unavailable");
    health.trouble = Trouble::Clear;
    health.local_height = None;
    assert_eq!(health.status(), "No locally indexed blocks");
}

#[test]
fn endpoint_never_displays_credentials_or_path() {
    assert_eq!(
        safe_endpoint("https://example.org/secret?key=x"),
        "https://example.org"
    );
    assert_eq!(
        safe_endpoint("https://user:pass@example.org/path"),
        "Endpoint hidden"
    );
    assert_eq!(safe_endpoint("https://example.org:8443/api"), "https://example.org:8443");
    assert_eq!(safe_endpoint("https://example.org:/api"), "Endpoint hidden");
    assert_eq!(safe_endpoint("ftp://example.org"), "Endpoint hidden");
}
