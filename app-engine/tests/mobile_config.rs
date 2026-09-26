use wire_app_engine::config::Settings;

#[test]
fn mobile_settings_use_only_the_app_private_identity_path() {
    let app_data_dir = tempfile::tempdir().unwrap();
    let settings = Settings::mobile(app_data_dir.path());

    assert_eq!(
        settings.vault.as_deref(),
        Some(app_data_dir.path().join("identity.urma").as_path())
    );
    assert!(settings.unlock_file.is_none());
    assert!(settings.rpc_url.is_none());
    assert!(settings.node_auth_file.is_none());
}
