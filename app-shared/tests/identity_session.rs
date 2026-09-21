use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_shared::{config::Settings, identity::MAX_IDENTITY_VAULT_BYTES, live::Service};

#[test]
fn session_and_import_preserve_identity_without_a_working_source() {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let keys = Keyring::create(&phrase, "public fixture").unwrap();
    let vault = UnlockedVault::create(keys, "fixture-password", &mut rand::rngs::OsRng).unwrap();
    let bytes = vault
        .seal(&mut rand::rngs::OsRng)
        .unwrap()
        .as_bytes()
        .to_vec();
    let original = dir.path().join("cli.urma");
    std::fs::write(&original, &bytes).unwrap();
    let settings = || Settings {
        vault: Some(original.clone()),
        // An incomplete source config fails locally, without making network requests.
        rpc_url: Some("http://127.0.0.1:1".into()),
        ..Default::default()
    };
    let app = dir.path().join("app");
    let mut service = Service::open(&app, settings()).unwrap();
    let locked = service.identity_status().unwrap();
    assert!(!locked.unlocked);
    assert!(locked.vault_available);
    assert_eq!(locked.source, "configured");
    assert!(locked.author_pubkey.is_none());
    service.unlock("fixture-password".into(), false).unwrap();
    let active = service.identity_status().unwrap();
    assert!(active.unlocked);
    assert!(active.author_pubkey.is_some());
    assert!(service.wallet().is_err());
    assert_eq!(active, service.identity_status().unwrap());
    assert!(service.unlock("wrong".into(), false).is_err());
    assert_eq!(active, service.identity_status().unwrap());
    assert!(service
        .import_identity(bytes.clone(), "wrong".into())
        .is_err());
    assert!(!app.join("identity.urma").exists());
    assert!(service
        .import_identity(
            vec![0; MAX_IDENTITY_VAULT_BYTES + 1],
            "fixture-password".into()
        )
        .is_err());
    assert_eq!(active, service.identity_status().unwrap());
    let imported = service
        .import_identity(bytes.clone(), "fixture-password".into())
        .unwrap();
    assert_eq!(imported.source, "imported");
    assert_eq!(active.author_pubkey, imported.author_pubkey);
    assert!(service
        .import_identity(vec![0; 300], "fixture-password".into())
        .is_err());
    assert_eq!(std::fs::read(app.join("identity.urma")).unwrap(), bytes);
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    assert_eq!(imported, service.identity_status().unwrap());
    service.lock();
    assert!(!service.identity_status().unwrap().unlocked);
    assert!(service.identity_status().unwrap().author_pubkey.is_none());
    service.unlock("fixture-password".into(), false).unwrap();
    drop(service);
    let service = Service::open(&app, settings()).unwrap();
    assert!(!service.identity_status().unwrap().unlocked);
    assert_eq!(service.identity_status().unwrap().source, "imported");
    assert_eq!(
        MAX_IDENTITY_VAULT_BYTES,
        urma_identity::vault::EncryptedVault::MAX_BYTES
    );
}
