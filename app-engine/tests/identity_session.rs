use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_core::config::MAX_VAULT_BYTES;
use wire_app_core::connection::SourceChoice;
use wire_app_core::session::{Credential, Session, Vault};
use wire_app_engine::config::Settings;
use wire_app_engine::native::service::Service;

fn password(text: &str) -> Credential {
    Credential::Password(text.into())
}

fn offline() -> Settings {
    Settings {
        rpc_url: Some("http://127.0.0.1:1".into()),
        source: SourceChoice::LocalNode,
        ..Default::default()
    }
}

fn author(session: Session) -> wire_app_core::model::AuthorId {
    let Session::Unlocked(author) = session else {
        panic!("expected an unlocked session");
    };
    author
}

#[test]
fn create_lock_unlock_and_restart_follow_one_status() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app");
    let service = Service::open(&app, offline()).unwrap();
    let fresh = service.identity_status().unwrap();
    assert_eq!(fresh.session, Session::Locked);
    assert_eq!(fresh.vault, Vault::Missing);
    assert!(service.unlock(password("fixture-password")).is_err());
    assert!(service.create_identity("short".into(), false).is_err());
    assert!(!app.join("identity-v2.urma").exists());

    let created = service
        .create_identity("fixture-password".into(), false)
        .unwrap();
    assert_eq!(created.words.split_whitespace().count(), 12);
    let me = author(created.status.session);
    assert_eq!(created.status.vault, Vault::Local);
    assert_eq!(created.status, service.identity_status().unwrap());

    assert!(service.wallet().is_err());
    assert_eq!(created.status, service.identity_status().unwrap());

    let locked = service.lock().unwrap();
    assert_eq!(locked.session, Session::Locked);
    assert_eq!(locked.vault, Vault::Local);
    assert!(service.unlock(password("wrong-password")).is_err());
    assert_eq!(locked, service.identity_status().unwrap());
    let unlocked = service.unlock(password("fixture-password")).unwrap();
    assert_eq!(unlocked.session, Session::Unlocked(me));

    drop(service);
    let restarted = Service::open(&app, offline()).unwrap();
    let status = restarted.identity_status().unwrap();
    assert_eq!(status.session, Session::Locked);
    assert_eq!(status.vault, Vault::Local);
    assert_eq!(MAX_VAULT_BYTES, urma_identity::vault::EncryptedVault::MAX_BYTES);
}

#[test]
fn twelve_words_recover_the_identity_without_the_old_password() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app");
    let service = Service::open(&app, offline()).unwrap();
    let created = service
        .create_identity("forgotten-password".into(), false)
        .unwrap();
    let me = author(created.status.session);
    let words = created.words.to_string();
    let before = std::fs::read(app.join("identity-v2.urma")).unwrap();
    service.lock().unwrap();

    assert!(service
        .recover_identity(words.clone(), "new-device-password".into(), false)
        .is_err());
    assert_eq!(std::fs::read(app.join("identity-v2.urma")).unwrap(), before);
    assert_eq!(service.identity_status().unwrap().session, Session::Locked);
    assert!(service
        .recover_identity("not twelve words".into(), "new-device-password".into(), true)
        .is_err());
    assert_eq!(std::fs::read(app.join("identity-v2.urma")).unwrap(), before);

    let recovered = service
        .recover_identity(format!("  {words} \n"), "new-device-password".into(), true)
        .unwrap();
    assert_eq!(recovered.session, Session::Unlocked(me));
    assert_eq!(recovered.vault, Vault::Local);
    assert_ne!(std::fs::read(app.join("identity-v2.urma")).unwrap(), before);

    service.lock().unwrap();
    assert!(service.unlock(password("forgotten-password")).is_err());
    assert_eq!(
        service.unlock(password("new-device-password")).unwrap().session,
        Session::Unlocked(me)
    );

    let elsewhere = Service::open(&dir.path().join("elsewhere"), offline()).unwrap();
    let again = elsewhere
        .recover_identity(words.clone(), "third-password".into(), false)
        .unwrap();
    assert_eq!(again.session, Session::Unlocked(me));
    let expected = Keyring::recover(&IdentityPhrase::parse(&words).unwrap()).unwrap();
    assert_eq!(expected.active().unwrap().author().0.serialize(), me.0);
}

#[test]
fn a_configured_cli_vault_unlocks_and_an_import_replaces_only_when_told() {
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
        ..offline()
    };
    let app = dir.path().join("app");
    let service = Service::open(&app, settings()).unwrap();
    let configured = service.identity_status().unwrap();
    assert_eq!(configured.session, Session::Locked);
    assert_eq!(
        configured.vault,
        Vault::Configured {
            path: original.display().to_string(),
            unlock_file: None,
        }
    );
    let cli = author(service.unlock(password("fixture-password")).unwrap().session);

    assert!(service
        .import_identity(bytes.clone(), "wrong".into(), false)
        .is_err());
    assert!(!app.join("identity-v2.urma").exists());
    assert!(service
        .import_identity(vec![0; MAX_VAULT_BYTES + 1], "fixture-password".into(), false)
        .is_err());
    let imported = service
        .import_identity(bytes.clone(), "fixture-password".into(), false)
        .unwrap();
    assert_eq!(imported.session, Session::Unlocked(cli));
    assert_eq!(imported.vault, Vault::Local);
    assert_eq!(std::fs::read(app.join("identity-v2.urma")).unwrap(), bytes);
    assert_eq!(std::fs::read(&original).unwrap(), bytes);

    assert!(service
        .import_identity(bytes.clone(), "fixture-password".into(), false)
        .is_err());
    let other = service
        .create_identity("another-password".into(), true)
        .unwrap();
    assert_ne!(author(other.status.session), cli);
    assert_ne!(std::fs::read(app.join("identity-v2.urma")).unwrap(), bytes);
    let back = service
        .import_identity(bytes.clone(), "fixture-password".into(), true)
        .unwrap();
    assert_eq!(back.session, Session::Unlocked(cli));
    assert_eq!(std::fs::read(app.join("identity-v2.urma")).unwrap(), bytes);
}
