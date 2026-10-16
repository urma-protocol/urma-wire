use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_core::config::MAX_VAULT_BYTES;
use wire_app_core::connection::SourceChoice;
use wire_app_core::session::{Credential, Session, VaultSource};
use wire_app_engine::config::Settings;
use wire_app_engine::native::service::Service;

fn password(text: &str) -> Credential {
    Credential::Password(text.into())
}

#[test]
fn session_and_import_preserve_identity_without_a_working_source() {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}about", "abandon ".repeat(11))).unwrap();
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
        rpc_url: Some("http://127.0.0.1:1".into()),
        source: SourceChoice::LocalNode,
        ..Default::default()
    };
    let app = dir.path().join("app");
    let mut service = Service::open(&app, settings()).unwrap();
    let locked = service.identity_status().unwrap();
    assert_eq!(locked.session, Session::Locked);
    assert!(locked.vault_available);
    assert_eq!(locked.source, VaultSource::Configured);
    service.unlock(password("fixture-password")).unwrap();
    let active = service.identity_status().unwrap();
    assert!(matches!(active.session, Session::Unlocked(_)));
    assert!(service.wallet().is_err());
    assert_eq!(active, service.identity_status().unwrap());
    assert!(service.unlock(password("wrong")).is_err());
    assert_eq!(active, service.identity_status().unwrap());
    assert!(service
        .import_identity(bytes.clone(), "wrong".into())
        .is_err());
    assert!(!app.join("identity-v2.urma").exists());
    assert!(service
        .import_identity(vec![0; MAX_VAULT_BYTES + 1], "fixture-password".into())
        .is_err());
    assert_eq!(active, service.identity_status().unwrap());
    let imported = service
        .import_identity(bytes.clone(), "fixture-password".into())
        .unwrap();
    assert_eq!(imported.source, VaultSource::Imported);
    assert_eq!(active.session, imported.session);
    assert!(service
        .import_identity(vec![0; 300], "fixture-password".into())
        .is_err());
    assert_eq!(std::fs::read(app.join("identity-v2.urma")).unwrap(), bytes);
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    assert_eq!(imported, service.identity_status().unwrap());
    service.lock();
    assert_eq!(service.identity_status().unwrap().session, Session::Locked);
    service.unlock(password("fixture-password")).unwrap();
    drop(service);
    let service = Service::open(&app, settings()).unwrap();
    assert_eq!(service.identity_status().unwrap().session, Session::Locked);
    assert_eq!(
        service.identity_status().unwrap().source,
        VaultSource::Imported
    );
    assert_eq!(
        MAX_VAULT_BYTES,
        urma_identity::vault::EncryptedVault::MAX_BYTES
    );
}

#[test]
fn a_created_identity_is_recoverable_from_its_twelve_words() {
    let dir = tempfile::tempdir().unwrap();
    let settings = || Settings {
        rpc_url: Some("http://127.0.0.1:1".into()),
        source: SourceChoice::LocalNode,
        ..Default::default()
    };
    let first = Service::open(&dir.path().join("first"), settings()).unwrap();
    assert!(first
        .create_identity("not twelve words".into(), "fixture-password".into())
        .is_err());
    assert!(first
        .create_identity(String::new(), "short".into())
        .is_err());
    assert!(!dir.path().join("first/identity-v2.urma").exists());
    let (status, words) = first
        .create_identity(String::new(), "fixture-password".into())
        .unwrap();
    assert_eq!(words.split_whitespace().count(), 12);
    let Session::Unlocked(author) = status.session else {
        panic!("a created identity is unlocked");
    };
    assert_eq!(status.source, VaultSource::Imported);
    assert!(first
        .create_identity(String::new(), "fixture-password".into())
        .is_err());
    first.lock();
    first.unlock(password("fixture-password")).unwrap();
    assert_eq!(first.identity_status().unwrap().session, Session::Unlocked(author));
    let second = Service::open(&dir.path().join("second"), settings()).unwrap();
    let (recovered, same_words) = second
        .create_identity(format!("  {words} \n"), "another-password".into())
        .unwrap();
    assert_eq!(recovered.session, Session::Unlocked(author));
    assert_eq!(same_words, words);
    let expected = IdentityPhrase::parse(&words).unwrap();
    let keys = Keyring::recover(&expected).unwrap();
    assert_eq!(
        keys.active().unwrap().author().0.serialize(),
        author.0
    );
}
