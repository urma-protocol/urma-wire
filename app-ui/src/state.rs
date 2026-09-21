use crate::platform::tauri_call;
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{
    connection::ConnectionHealth,
    live_model::{Feed, Publication},
    AuthorId, Chain, IdentityStatus, Post, WalletSummary,
};

fn check_time_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub active_chain: RwSignal<Chain>,
    pub autoload_images: RwSignal<bool>,
    pub media_epoch: RwSignal<u64>,
    pub media_inflight: RwSignal<usize>,
    pub media_bytes: RwSignal<usize>,
    pub wallet_summary: RwSignal<Option<WalletSummary>>,
    pub identity: RwSignal<Option<IdentityStatus>>,
    pub identity_error: RwSignal<Option<String>>,
    pub feed: RwSignal<Vec<Post>>,
    pub post_draft: RwSignal<String>,
    pub public_identities: RwSignal<
        std::collections::BTreeMap<String, wire_app_shared::public_identity::PublicIdentity>,
    >,
    pub is_loading_feed: RwSignal<bool>,
    pub is_loading_wallet: RwSignal<bool>,
    pub status_message: RwSignal<Option<String>>,
    pub feed_status: RwSignal<String>,
    pub publication: RwSignal<Option<Publication>>,
    pub following: RwSignal<bool>,
    pub pause_requested: RwSignal<bool>,
    pub publication_error: RwSignal<Option<String>>,
    pub connection_health: RwSignal<Option<ConnectionHealth>>,
    pub connection_last_success_ms: RwSignal<Option<u64>>,
    pub is_checking_connection: RwSignal<bool>,
}

impl AppState {
    pub fn new() -> Self {
        let image_preference = crate::components::markdown::load_autoload();
        let state = Self {
            active_chain: RwSignal::new(Chain::Litecoin),
            autoload_images: RwSignal::new(image_preference.clone().unwrap_or(false)),
            media_epoch: RwSignal::new(0),
            media_inflight: RwSignal::new(0),
            media_bytes: RwSignal::new(0),
            wallet_summary: RwSignal::new(None),
            identity: RwSignal::new(None),
            identity_error: RwSignal::new(None),
            feed: RwSignal::new(Vec::new()),
            post_draft: RwSignal::new(String::new()),
            public_identities: RwSignal::new(Default::default()),
            is_loading_feed: RwSignal::new(false),
            is_loading_wallet: RwSignal::new(false),
            status_message: RwSignal::new(image_preference.err()),
            feed_status: RwSignal::new(String::new()),
            publication: RwSignal::new(None),
            following: RwSignal::new(false),
            pause_requested: RwSignal::new(false),
            publication_error: RwSignal::new(None),
            connection_health: RwSignal::new(None),
            connection_last_success_ms: RwSignal::new(None),
            is_checking_connection: RwSignal::new(false),
        };
        state.load_identity();
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
        self.public_identities.set(Default::default());
        self.wallet_summary.set(None);
        self.connection_health.set(None);
        self.connection_last_success_ms.set(None);
        self.refresh_all();
    }

    pub fn check_connection(&self) {
        if self.is_checking_connection.get_untracked() {
            return;
        }
        let state = *self;
        let chain = state.active_chain.get_untracked();
        state.is_checking_connection.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<ConnectionHealth>(
                "get_connection_health",
                &json!({ "chain": chain.code() }),
            )
            .await
            {
                Ok(mut health) => {
                    if health.remote_height.is_some() {
                        state
                            .connection_last_success_ms
                            .set(Some(health.checked_at_ms));
                    } else if health.remote_height.is_none() {
                        if let Some(previous) = state.connection_health.get_untracked() {
                            if previous.endpoint == health.endpoint
                                && previous.source_type == health.source_type
                            {
                                health.remote_height = previous.remote_height;
                            }
                        }
                    }
                    state.connection_health.set(Some(health));
                }
                Err(_) => {
                    let mut health =
                        state
                            .connection_health
                            .get_untracked()
                            .unwrap_or(ConnectionHealth {
                                network: chain.network_name().into(),
                                source_type: "Unknown".into(),
                                endpoint: "Endpoint hidden".into(),
                                checked_at_ms: 0,
                                remote_height: None,
                                local_start: None,
                                local_height: None,
                                error: None,
                            });
                    health.checked_at_ms = check_time_ms();
                    health.error = Some("Connection check could not complete".into());
                    state.connection_health.set(Some(health));
                }
            }
            state.is_checking_connection.set(false);
        });
    }

    pub fn load_identity(&self) {
        let state = *self;
        leptos::task::spawn_local(async move {
            match tauri_call::<IdentityStatus>("identity_status", &json!({})).await {
                Ok(status) => {
                    let unlocked = status.unlocked;
                    state.identity.set(Some(status));
                    state.identity_error.set(None);
                    if unlocked {
                        state.load_wallet();
                    } else {
                        state.wallet_summary.set(None);
                    }
                }
                Err(error) => state.identity_error.set(Some(error)),
            }
        });
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
                    if state.active_chain.get_untracked() != chain {
                        state.is_loading_feed.set(false);
                        return;
                    }
                    state.public_identities.set(result.identities);
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
        let author = state
            .identity
            .get_untracked()
            .and_then(|status| status.author_pubkey);
        leptos::task::spawn_local(async move {
            let result = tauri_call::<WalletSummary>(
                "get_wallet_summary",
                &json!({ "chain": chain.code() }),
            )
            .await;
            if state
                .identity
                .get_untracked()
                .and_then(|status| status.author_pubkey)
                != author
            {
                state.is_loading_wallet.set(false);
                return;
            }
            match result {
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
        self.identity
            .get()
            .and_then(|status| status.author_pubkey)
            .and_then(|author| AuthorId::from_hex(&author).ok())
    }

    pub fn display_name(&self, author: AuthorId) -> String {
        self.public_identities
            .with(|identities| {
                identities
                    .get(&author.to_hex())
                    .and_then(|identity| identity.nickname.as_ref())
                    .filter(|name| !name.is_empty())
                    .cloned()
            })
            .unwrap_or_else(|| author.fingerprint())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
