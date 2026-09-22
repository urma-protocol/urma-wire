use crate::components::markdown::stored_autoload;
use crate::components::Said;
use crate::data::chain;
use crate::data::failure::Failure;
use crate::data::identity;
use crate::data::publication;
use crate::data::publication::Pending;
use crate::data::records;
use crate::limits::PUBLICATION_POLL_MS;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::With;
use std::collections::BTreeMap;
use tracing::warn;
use wire_app_shared::connection::ConnectionHealth;
use wire_app_shared::live_model::Publication;
use wire_app_shared::public_identity::PublicIdentity;
use wire_app_shared::AuthorId;
use wire_app_shared::Chain;
use wire_app_shared::IdentityStatus;
use wire_app_shared::Post;
use wire_app_shared::WalletSummary;

#[derive(Clone, Copy)]
pub struct AppState {
    pub active_chain: RwSignal<Chain>,
    pub autoload_images: RwSignal<bool>,
    pub media_epoch: RwSignal<u64>,
    pub media_inflight: RwSignal<usize>,
    pub media_bytes: RwSignal<usize>,
    pub wallet_summary: RwSignal<Option<WalletSummary>>,
    pub identity: RwSignal<Option<IdentityStatus>>,
    pub identity_error: RwSignal<Said>,
    pub feed: RwSignal<Vec<Post>>,
    pub post_draft: RwSignal<String>,
    pub public_identities: RwSignal<BTreeMap<String, PublicIdentity>>,
    pub is_loading_feed: RwSignal<bool>,
    pub is_loading_wallet: RwSignal<bool>,
    pub status_message: RwSignal<Said>,
    pub feed_status: RwSignal<String>,
    pub publication: RwSignal<Option<Publication>>,
    pub following: RwSignal<bool>,
    pub pause_requested: RwSignal<bool>,
    pub publication_error: RwSignal<Said>,
    pub connection_health: RwSignal<Option<ConnectionHealth>>,
    pub connection_last_success_ms: RwSignal<Option<u64>>,
    pub is_checking_connection: RwSignal<bool>,
}

impl AppState {
    pub fn started() -> Self {
        let preference = stored_autoload();
        let state = Self {
            active_chain: RwSignal::new(Chain::Litecoin),
            autoload_images: RwSignal::new(preference.wanted()),
            media_epoch: RwSignal::new(0),
            media_inflight: RwSignal::new(0),
            media_bytes: RwSignal::new(0),
            wallet_summary: RwSignal::new(None),
            identity: RwSignal::new(None),
            identity_error: RwSignal::new(Said::Nothing),
            feed: RwSignal::new(Vec::new()),
            post_draft: RwSignal::new(String::new()),
            public_identities: RwSignal::new(BTreeMap::new()),
            is_loading_feed: RwSignal::new(false),
            is_loading_wallet: RwSignal::new(false),
            status_message: RwSignal::new(preference.complaint()),
            feed_status: RwSignal::new(String::new()),
            publication: RwSignal::new(None),
            following: RwSignal::new(false),
            pause_requested: RwSignal::new(false),
            publication_error: RwSignal::new(Said::Nothing),
            connection_health: RwSignal::new(None),
            connection_last_success_ms: RwSignal::new(None),
            is_checking_connection: RwSignal::new(false),
        };
        state.load_identity();
        state.load_feed();
        state.load_pending_publication();
        state
    }

    fn load_pending_publication(&self) {
        let state = *self;
        leptos::task::spawn_local(async move {
            match publication::pending().await {
                Ok(Pending::Awaiting(found)) => state.publication.set(Some(found)),
                Ok(Pending::Settled) => state.publication.set(None),
                Err(failure) => {
                    warn!(failure = %failure, "pending publication unavailable");
                    state.publication_error.set(state.voiced(failure));
                }
            }
        });
    }

    fn voiced(&self, failure: Failure) -> Said {
        Said::Something(failure.reported())
    }

    pub fn set_chain(&self, chain: Chain) {
        self.active_chain.set(chain);
        self.feed.set(Vec::new());
        self.public_identities.set(BTreeMap::new());
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
            match chain::health(chain).await {
                Ok(health) => state.connection_health.set(Some(carried(health, state))),
                Err(failure) => {
                    warn!(failure = %failure, "connection check could not complete");
                    state
                        .connection_health
                        .set(Some(unreachable(chain, failure)));
                }
            }
            state.is_checking_connection.set(false);
        });
    }

    pub fn load_identity(&self) {
        let state = *self;
        leptos::task::spawn_local(async move {
            match identity::status().await {
                Ok(status) => {
                    let unlocked = status.unlocked;
                    state.identity.set(Some(status));
                    state.identity_error.set(Said::Nothing);
                    match unlocked {
                        true => state.load_wallet(),
                        false => state.wallet_summary.set(None),
                    }
                }
                Err(failure) => {
                    warn!(failure = %failure, "identity status unavailable");
                    state.identity_error.set(state.voiced(failure));
                }
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
            match records::feed(chain).await {
                Ok(result) => {
                    if state.active_chain.get_untracked() == chain {
                        state.public_identities.set(result.identities);
                        state.feed.set(result.posts);
                        state.feed_status.set(indexed_range(
                            result.start,
                            result.tip,
                            result.complete_to_tip,
                        ));
                    }
                }
                Err(failure) => {
                    warn!(failure = %failure, "feed sync failed; cached posts stand");
                    state.feed_status.set(sync_failed(failure));
                }
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
        let author = signed_by(state);
        leptos::task::spawn_local(async move {
            let result = chain::wallet(chain).await;
            if signed_by(state) != author {
                state.is_loading_wallet.set(false);
                return;
            }
            match result {
                Ok(summary) => {
                    state.wallet_summary.set(Some(summary));
                    state.status_message.set(Said::Nothing);
                }
                Err(failure) => {
                    warn!(failure = %failure, "funding summary unavailable");
                    state.wallet_summary.set(None);
                    state.status_message.set(state.voiced(failure));
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
        state.publication_error.set(Said::Nothing);
        leptos::task::spawn_local(async move {
            let mut approval = approve;
            while !state.pause_requested.get_untracked() {
                match publication::advance(&id, approval).await {
                    Ok(result) => {
                        let complete = result.complete;
                        state.publication.set(Some(result));
                        if complete {
                            state.refresh_all();
                            break;
                        }
                    }
                    Err(failure) => {
                        warn!(failure = %failure, "publication paused on a source failure");
                        state.publication_error.set(state.voiced(failure));
                        break;
                    }
                }
                approval = false;
                gloo_timers::future::TimeoutFuture::new(PUBLICATION_POLL_MS).await;
            }
            state.following.set(false);
        });
    }

    pub fn display_name(&self, author: AuthorId) -> String {
        let named = self.public_identities.with(|identities| {
            let Some(identity) = identities.get(&author.to_hex()) else {
                return None;
            };
            let Some(nickname) = identity.nickname.as_ref() else {
                return None;
            };
            match nickname.is_empty() {
                true => None,
                false => Some(nickname.clone()),
            }
        });
        let Some(nickname) = named else {
            return author.fingerprint();
        };
        nickname
    }
}

pub enum Signer {
    Author(AuthorId),
    Locked,
}

pub fn signing_author(state: AppState) -> Signer {
    let Some(status) = state.identity.get() else {
        return Signer::Locked;
    };
    let Some(author) = status.author_pubkey else {
        return Signer::Locked;
    };
    match AuthorId::from_hex(&author) {
        Ok(id) => Signer::Author(id),
        Err(error) => {
            warn!(error = %error, "session author key is not valid hex");
            Signer::Locked
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Holder {
    Key(String),
    None,
}

fn signed_by(state: AppState) -> Holder {
    let Some(status) = state.identity.get_untracked() else {
        return Holder::None;
    };
    let Some(author) = status.author_pubkey else {
        return Holder::None;
    };
    Holder::Key(author)
}

fn indexed_range(start: u64, tip: u64, complete: bool) -> String {
    let tail = match complete {
        true => "Up to date at last refresh.",
        false => "More history remains; refresh to continue.",
    };
    format!("Verified history from block {start} through {tip}. {tail}")
}

fn sync_failed(failure: Failure) -> String {
    format!(
        "Sync failed; displayed posts are cached: {}",
        failure.reported()
    )
}

fn carried(mut health: ConnectionHealth, state: AppState) -> ConnectionHealth {
    let Some(height) = health.remote_height else {
        let Some(previous) = state.connection_health.get_untracked() else {
            return health;
        };
        if previous.endpoint == health.endpoint && previous.source_type == health.source_type {
            health.remote_height = previous.remote_height;
        }
        return health;
    };
    health.remote_height = Some(height);
    state
        .connection_last_success_ms
        .set(Some(health.checked_at_ms));
    health
}

fn unreachable(chain: Chain, failure: Failure) -> ConnectionHealth {
    ConnectionHealth {
        network: chain.network_name().to_owned(),
        source_type: "Unknown".to_owned(),
        endpoint: "Endpoint hidden".to_owned(),
        checked_at_ms: 0,
        remote_height: None,
        local_start: None,
        local_height: None,
        error: Some(failure.reported()),
    }
}
