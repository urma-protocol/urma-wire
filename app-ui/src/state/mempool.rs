use crate::config::MEMPOOL_POLL_MS;
use crate::data::preferences::remember_show_mempool;
use crate::data::records;
use crate::state::said::Said;
use crate::state::session::AppState;
use leptos::prelude::GetUntracked;
use leptos::prelude::Set;
use std::collections::BTreeSet;
use tracing::warn;
use wire_app_core::model::{Post, Txid};

impl AppState {
    pub fn set_show_mempool(&self, shown: bool) {
        match remember_show_mempool(shown) {
            Ok(()) => (),
            Err(fault) => {
                warn!(fault = %fault, "mempool preference could not be stored");
                self.status_message.set(Said::Something(fault.message));
            }
        }
        self.show_mempool.set(shown);
        self.mempool_said.set(Said::Nothing);
        if !shown {
            self.pending.set(Vec::new());
        }
    }

    pub fn adopt_pending(&self, posts: Vec<Post>) -> Departed {
        let listed: BTreeSet<Txid> = posts.iter().map(|post| post.txid).collect();
        let departed = self
            .pending
            .get_untracked()
            .iter()
            .any(|post| !listed.contains(&post.txid));
        self.pending.set(posts);
        self.mempool_said.set(Said::Nothing);
        match departed {
            true => Departed::Left,
            false => Departed::Stayed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Departed {
    Left,
    Stayed,
}

pub async fn watch_mempool(state: AppState) {
    loop {
        if state.show_mempool.get_untracked() {
            let chain = state.active_chain.get_untracked();
            match records::mempool(chain).await {
                Ok(posts) => match state.adopt_pending(posts) {
                    Departed::Left => state.refresh_feed(),
                    Departed::Stayed => (),
                },
                Err(fault) => {
                    warn!(fault = %fault, "mempool unavailable from the current sources");
                    state.mempool_said.set(Said::Something(fault.message));
                }
            }
        }
        gloo_timers::future::TimeoutFuture::new(MEMPOOL_POLL_MS).await;
    }
}
