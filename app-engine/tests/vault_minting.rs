use urma_identity::error::IdentityError;
use urma_identity::vault::{EncryptedVault, UnlockCredential};
use urma_wallet::account::Account;
use wire_app_engine::config::NETWORK;
use wire_app_engine::vault::{minted, Minted, Words};
use zeroize::Zeroizing;

fn author(minted: &Minted) -> Result<[u8; 32], IdentityError> {
    Ok(minted.vault.keyring().active()?.author().0.serialize())
}

#[test]
fn the_same_words_mint_the_same_keys_under_any_password() {
    let first = minted(Words::Fresh, "first-password").unwrap();
    assert_eq!(first.words.split_whitespace().count(), 12);
    let second = minted(
        Words::Given(Zeroizing::new(format!("  {}\n", first.words.as_str()))),
        "second-password",
    )
    .unwrap();
    assert_eq!(author(&first).unwrap(), author(&second).unwrap());
    assert_eq!(first.words.as_str(), second.words.as_str());
    assert_ne!(first.sealed, second.sealed);
    let funding = |minted: &Minted| {
        Account::from_root(NETWORK, minted.vault.keyring().root())
            .unwrap()
            .receive_address()
            .unwrap()
    };
    assert_eq!(funding(&first), funding(&second));

    let sealed = EncryptedVault::from_bytes(&second.sealed).unwrap();
    assert!(sealed
        .unlock(UnlockCredential::Password("first-password"))
        .is_err());
    let reopened = sealed
        .unlock(UnlockCredential::Password("second-password"))
        .unwrap();
    assert_eq!(
        reopened.keyring().active().unwrap().author().0.serialize(),
        author(&first).unwrap()
    );
}

#[test]
fn bad_words_and_short_passwords_mint_nothing() {
    assert!(minted(Words::Given(Zeroizing::new("eleven words only".into())), "long-enough-pw").is_err());
    assert!(minted(Words::Fresh, "short").is_err());
    let other = minted(Words::Fresh, "first-password").unwrap();
    let first = minted(Words::Fresh, "first-password").unwrap();
    assert_ne!(author(&first).unwrap(), author(&other).unwrap());
}
