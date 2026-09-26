use urma_identity::{identity::IdentitySigner, phrase::IdentityPhrase, vault::UnlockCredential};
use wire_app_core::model::Chain;
use wire_app_engine::native::wallet::{Balance, WireWallet};

fn shared_wallet() -> WireWallet {
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    WireWallet::unlock(
        include_bytes!("fixtures/valid.vault"),
        UnlockCredential::RecoveryPhrase(&phrase),
    )
    .unwrap()
}

#[test]
fn author_and_funding_are_the_same_active_identity() {
    let wallet = shared_wallet();
    let author = wallet.author_id().unwrap();
    let key = wallet.signer().unwrap();
    assert_eq!(
        author.0,
        key.public_key().inner.x_only_public_key().0.serialize()
    );
    assert_eq!(
        wallet.receive_address(Chain::Bitcoin).unwrap(),
        urma_wallet::address::receive_address(
            &key,
            urma_chain::observation::Chain::BitcoinTestnet4
        )
        .unwrap()
    );
    assert!(wallet
        .receive_address(Chain::Litecoin)
        .unwrap()
        .starts_with("tltc1q"));
    let summary = wallet
        .summary(
            Chain::Bitcoin,
            Balance {
                confirmed: 1000,
                unconfirmed: 0,
                outputs: 1,
                tip_height: 150_000,
            },
        )
        .unwrap();
    assert_eq!(summary.author_pubkey, author.to_hex());
    assert_eq!(summary.author_fingerprint, author.fingerprint());
    assert_eq!(summary.confirmed_balance_sats, 1000);
    assert_eq!(summary.utxo_count, 1);
}
