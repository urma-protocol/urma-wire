use crate::platform::tauri_call;
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{AuthorId, Chain, Post, WalletSummary};

#[derive(Clone, Copy)]
pub struct AppState {
    pub active_chain: RwSignal<Chain>,
    pub autoload_images: RwSignal<bool>,
    pub wallet_summary: RwSignal<Option<WalletSummary>>,
    pub feed: RwSignal<Vec<Post>>,
    pub is_loading_feed: RwSignal<bool>,
    pub is_loading_wallet: RwSignal<bool>,
    pub status_message: RwSignal<Option<String>>,
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
        };

        state.refresh_all();
        state
    }

    pub fn set_chain(&self, chain: Chain) {
        self.active_chain.set(chain);
        self.refresh_all();
    }

    pub fn refresh_all(&self) {
        self.load_feed();
        self.load_wallet();
    }

    pub fn load_feed(&self) {
        let chain = self.active_chain.get();
        let feed = self.feed;
        let loading = self.is_loading_feed;

        loading.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<Vec<Post>>(
                "get_feed",
                &json!({ "chain": chain.code(), "limit": 100 }),
            )
            .await
            {
                Ok(posts) => feed.set(posts),
                Err(e) => {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::warn_1(&format!("Failed to load feed: {e}").into());
                    let _ = e;
                }
            }
            loading.set(false);
        });
    }

    pub fn load_wallet(&self) {
        let chain = self.active_chain.get();
        let summary = self.wallet_summary;
        let loading = self.is_loading_wallet;

        loading.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<WalletSummary>(
                "get_wallet_summary",
                &json!({ "chain": chain.code() }),
            )
            .await
            {
                Ok(res) => summary.set(Some(res)),
                Err(e) => {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::warn_1(&format!("Failed to load wallet summary: {e}").into());
                    let _ = e;
                }
            }
            loading.set(false);
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
