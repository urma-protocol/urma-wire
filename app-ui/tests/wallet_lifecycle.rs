use leptos::prelude::{GetUntracked, Owner, Set};
use wire_app_core::errors::{Fault, FaultKind};
use wire_app_core::model::{AuthorId, Chain, WalletSummary};
use wire_app_core::session::{IdentityStatus, Session, Vault};
use wire_app_ui::state::{AppState, Identity, Said, WalletAnswer};

fn unlocked(byte: u8) -> Identity {
    Identity::Known(IdentityStatus {
        session: Session::Unlocked(AuthorId([byte; 32])),
        vault: Vault::Local,
    })
}

fn summary(tip: u64) -> WalletSummary {
    WalletSummary {
        author_pubkey: "aa".repeat(32),
        author_fingerprint: "aaaa".into(),
        receive_address: "tltc1qexample".into(),
        chain: Chain::Litecoin,
        network_name: Chain::Litecoin.network_name().to_owned(),
        confirmed_balance_sats: 5_000,
        unconfirmed_balance_sats: 0,
        utxo_count: 1,
        tip_height: tip,
    }
}

#[test]
fn a_stale_wallet_answer_is_discarded_and_the_current_identity_is_served() {
    Owner::new().with(|| {
        let state = AppState::blank();
        state.identity.set(unlocked(1));
        let first = Session::Unlocked(AuthorId([1; 32]));
        state.identity.set(unlocked(2));
        state.is_loading_wallet.set(true);
        assert_eq!(
            state.settle_wallet(first, Ok(summary(10))),
            WalletAnswer::Stale
        );
        assert!(state.wallet_summary.get_untracked().is_none());
        assert!(!state.is_loading_wallet.get_untracked());

        let second = Session::Unlocked(AuthorId([2; 32]));
        state.is_loading_wallet.set(true);
        assert_eq!(
            state.settle_wallet(second, Ok(summary(11))),
            WalletAnswer::Shown
        );
        assert_eq!(
            state
                .wallet_summary
                .get_untracked()
                .map(|shown| shown.tip_height),
            Some(11)
        );
        assert!(!state.is_loading_wallet.get_untracked());
        assert!(matches!(
            state.status_message.get_untracked(),
            Said::Nothing
        ));
    });
}

#[test]
fn a_failed_wallet_answer_resets_the_flag_and_voices_the_fault() {
    Owner::new().with(|| {
        let state = AppState::blank();
        state.identity.set(unlocked(3));
        let current = Session::Unlocked(AuthorId([3; 32]));
        state.wallet_summary.set(Some(summary(1)));
        state.is_loading_wallet.set(true);
        let fault = Fault::new(FaultKind::Source, "Chain source unavailable");
        assert_eq!(
            state.settle_wallet(current, Err(fault)),
            WalletAnswer::Shown
        );
        assert!(state.wallet_summary.get_untracked().is_none());
        assert!(!state.is_loading_wallet.get_untracked());
        assert!(matches!(
            state.status_message.get_untracked(),
            Said::Something(message) if message == "Chain source unavailable"
        ));
    });
}

#[test]
fn a_locked_identity_never_requests_a_wallet() {
    Owner::new().with(|| {
        let state = AppState::blank();
        state.wallet_summary.set(Some(summary(1)));
        state.load_wallet();
        assert!(state.wallet_summary.get_untracked().is_none());
        assert!(!state.is_loading_wallet.get_untracked());
    });
}
