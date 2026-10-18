use crate::data::chain;
use crate::state::said::Said;
use crate::state::session::{voiced, AppState};
use leptos::prelude::GetUntracked;
use leptos::prelude::Set;
use tracing::warn;
use wire_app_core::errors::Fault;
use wire_app_core::model::WalletSummary;
use wire_app_core::session::Session;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalletAnswer {
    Shown,
    Stale,
}

impl AppState {
    pub fn load_wallet(&self) {
        let state = *self;
        let requested = signed_by(state);
        if matches!(requested, Session::Locked) {
            state.wallet_summary.set(None);
            state.is_loading_wallet.set(false);
            return;
        }
        if state.is_loading_wallet.get_untracked() {
            return;
        }
        let chain = state.active_chain.get_untracked();
        state.is_loading_wallet.set(true);
        leptos::task::spawn_local(async move {
            let result = chain::wallet(chain).await;
            match state.settle_wallet(requested, result) {
                WalletAnswer::Shown => (),
                WalletAnswer::Stale => state.load_wallet(),
            }
        });
    }

    pub fn settle_wallet(
        &self,
        requested: Session,
        result: Result<WalletSummary, Fault>,
    ) -> WalletAnswer {
        self.is_loading_wallet.set(false);
        if signed_by(*self) != requested {
            warn!("wallet answer belongs to a previous identity; discarded and re-requested");
            return WalletAnswer::Stale;
        }
        match result {
            Ok(summary) => {
                self.wallet_summary.set(Some(summary));
                self.status_message.set(Said::Nothing);
            }
            Err(fault) => {
                warn!(fault = %fault, "funding summary unavailable");
                self.wallet_summary.set(None);
                self.status_message.set(voiced(fault));
            }
        }
        WalletAnswer::Shown
    }

}

pub(crate) fn signed_by(state: AppState) -> Session {
    state.session_untracked()
}
