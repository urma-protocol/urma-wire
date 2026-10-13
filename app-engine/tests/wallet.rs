use bitcoin::{CompressedPublicKey, ScriptBuf};
use urma_identity::{identity::IdentitySigner, vault::UnlockCredential};
use wire_app_core::model::Chain;
use wire_app_engine::native::wallet::{Balance, WireWallet};

const PASSWORD: &str = "URMA public test password 2026";
const ACTIVE_AUTHOR: &str = "45c28418e2bdbc795087330ec40629f558a00b56882b4545df9d62ceb6f89add";
const LITECOIN_RECEIVE: &str = "tltc1q2cec2gekurckshaypcpwklk35ft5s0f9y950n0";
const BITCOIN_RECEIVE: &str = "tb1q2cec2gekurckshaypcpwklk35ft5s0f9adk3rx";

fn vector_wallet() -> WireWallet {
    WireWallet::unlock(
        include_bytes!("fixtures/valid.vault"),
        UnlockCredential::Password(PASSWORD),
    )
    .unwrap()
}

#[test]
fn author_is_the_active_slot_and_funding_is_the_bip84_account() {
    let wallet = vector_wallet();
    let author = wallet.author_id().unwrap();
    assert_eq!(author.to_hex(), ACTIVE_AUTHOR);
    assert_eq!(
        wallet.receive_address(Chain::Litecoin).unwrap(),
        LITECOIN_RECEIVE
    );
    assert_eq!(
        wallet.receive_address(Chain::Bitcoin).unwrap(),
        BITCOIN_RECEIVE
    );
    let funder = wallet.funder(Chain::Litecoin).unwrap();
    assert_eq!(funder.receive_address().unwrap(), LITECOIN_RECEIVE);
    let identity = CompressedPublicKey::try_from(wallet.signer().unwrap().public_key()).unwrap();
    assert_ne!(
        funder.receive_script().unwrap(),
        ScriptBuf::new_p2wpkh(&identity.wpubkey_hash())
    );
    let summary = wallet
        .summary(
            Chain::Litecoin,
            Balance {
                confirmed: 1000,
                unconfirmed: 0,
                outputs: 1,
                tip_height: 150_000,
            },
        )
        .unwrap();
    assert_eq!(summary.author_pubkey, ACTIVE_AUTHOR);
    assert_eq!(summary.author_fingerprint, author.fingerprint());
    assert_eq!(summary.receive_address, LITECOIN_RECEIVE);
    assert_eq!(summary.confirmed_balance_sats, 1000);
    assert_eq!(summary.utxo_count, 1);
}

#[test]
fn version_one_vault_bytes_are_refused() {
    let mut legacy = include_bytes!("fixtures/valid.vault").to_vec();
    legacy[8] = 1;
    assert!(WireWallet::unlock(&legacy, UnlockCredential::Password(PASSWORD)).is_err());
}
