use crate::platform::tauri_call;
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{
    live_model::{Feed, Publication},
    AuthorId, Chain, Post, WalletSummary,
};

#[derive(Clone, Copy)]
pub struct AppState {
    pub active_chain: RwSignal<Chain>,
    pub autoload_images: RwSignal<bool>,
    pub wallet_summary: RwSignal<Option<WalletSummary>>,
    pub feed: RwSignal<Vec<Post>>,
    pub is_loading_feed: RwSignal<bool>,
    pub is_loading_wallet: RwSignal<bool>,
    pub status_message: RwSignal<Option<String>>,
    pub feed_status: RwSignal<String>,
    pub publication: RwSignal<Option<Publication>>,
    pub following: RwSignal<bool>,
    pub pause_requested: RwSignal<bool>,
    pub publication_error: RwSignal<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        let state = Self {
            active_chain: RwSignal::new(Chain::Litecoin),
            autoload_images: RwSignal::new(true),
            wallet_summary: RwSignal::new(None),
            feed: RwSignal::new(Vec::new()),
            is_loading_feed: RwSignal::new(false),
            is_loading_wallet: RwSignal::new(false),
            status_message: RwSignal::new(None),
            feed_status: RwSignal::new(String::new()),
            publication: RwSignal::new(None),
            following: RwSignal::new(false),
            pause_requested: RwSignal::new(false),
            publication_error: RwSignal::new(None),
        };
        state.load_feed();
        leptos::task::spawn_local(async move {
            match tauri_call::<Option<Publication>>("publication_status", &json!({})).await {
                Ok(value) => state.publication.set(value),
                Err(error) => state.publication_error.set(Some(error)),
            }
        });
        state
    }

    pub fn set_chain(&self, chain: Chain) {
        self.active_chain.set(chain);
        self.feed.set(Vec::new());
        self.wallet_summary.set(None);
        self.refresh_all();
    }

    pub fn refresh_all(&self) {
        self.load_feed();
        self.load_wallet();
    }

    pub fn load_feed(&self) {
        if self.is_loading_feed.get_untracked() {
            return;
        }
        let state = *self;
        let chain = state.active_chain.get_untracked();
        state.is_loading_feed.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<Feed>("get_feed", &json!({ "chain": chain.code() })).await {
                Ok(result) => {
                    state.feed.set(result.posts);
                    state.feed_status.set(format!(
                        "Verified history from block {} through {}. {}",
                        result.start,
                        result.tip,
                        if result.complete_to_tip {
                            "Up to date at last refresh."
                        } else {
                            "More history remains; refresh to continue."
                        }
                    ));
                }
                Err(error) => state
                    .feed_status
                    .set(format!("Sync failed; displayed posts are cached: {error}")),
            }
            state.is_loading_feed.set(false);
        });
    }

    pub fn load_wallet(&self) {
        if self.is_loading_wallet.get_untracked() {
            return;
        }
        let state = *self;
        let chain = state.active_chain.get_untracked();
        state.is_loading_wallet.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<WalletSummary>(
                "get_wallet_summary",
                &json!({ "chain": chain.code() }),
            )
            .await
            {
                Ok(result) => {
                    state.wallet_summary.set(Some(result));
                    state.status_message.set(None);
                }
                Err(error) => {
                    state.wallet_summary.set(None);
                    state.status_message.set(Some(error));
                }
            }
            state.is_loading_wallet.set(false);
        });
    }

    pub fn follow(&self, id: String, approve: bool) {
        if self.following.get_untracked() {
            return;
        }
        let state = *self;
        state.following.set(true);
        state.pause_requested.set(false);
        state.publication_error.set(None);
        leptos::task::spawn_local(async move {
            let mut approval = approve;
            while !state.pause_requested.get_untracked() {
                match tauri_call::<Publication>(
                    "advance_publication",
                    &json!({ "id": id, "approve": approval }),
                )
                .await
                {
                    Ok(result) => {
                        let complete = result.complete;
                        state.publication.set(Some(result));
                        if complete {
                            state.refresh_all();
                            break;
                        }
                    }
                    Err(error) => {
                        state.publication_error.set(Some(error));
                        break;
                    }
                }
                approval = false;
                gloo_timers::future::TimeoutFuture::new(2000).await;
            }
            state.following.set(false);
        });
    }

    pub fn author_id(&self) -> Option<AuthorId> {
        self.wallet_summary
            .get()
            .and_then(|w| AuthorId::from_hex(&w.author_pubkey).ok())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
