use crate::config::PUBLICATION_POLL_MS;
use crate::data::chain;
use crate::data::identity;
use crate::data::preferences::{stored_autoload, stored_replies_shown};
use crate::data::publication;
use crate::data::records;
use crate::data::storage::Complaint;
use crate::data::topics;
use crate::data::topics::Remembered;
use crate::state::addressing::Speaking;
use crate::state::said::Said;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Update;
use leptos::prelude::With;
use leptos::prelude::WithUntracked;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tracing::warn;
use wire_app_core::composer::Photo;
use wire_app_core::connection::{ConnectionHealth, Trouble};
use wire_app_core::errors::Fault;
use wire_app_core::feed::{conversations, following, Conversation, PublicIdentity};
use wire_app_core::model::{AuthorId, Chain, Post, WalletSummary};
use wire_app_core::publication::{Addressed, Content, Pending, Publication};
use wire_app_core::session::{IdentityStatus, Session};
use wire_app_core::topics::seen_in;

#[derive(Clone, Copy)]
pub struct AppState {
    pub active_chain: RwSignal<Chain>,
    pub autoload_images: RwSignal<bool>,
    pub replies_shown: RwSignal<usize>,
    pub media_epoch: RwSignal<u64>,
    pub media_inflight: RwSignal<usize>,
    pub media_bytes: RwSignal<usize>,
    pub wallet_summary: RwSignal<Option<WalletSummary>>,
    pub identity: RwSignal<Option<IdentityStatus>>,
    pub identity_error: RwSignal<Said>,
    pub feed: RwSignal<Vec<Post>>,
    pub drafts: RwSignal<BTreeMap<String, String>>,
    pub draft_photos: RwSignal<BTreeMap<String, Vec<Photo>>>,
    pub draft_labels: RwSignal<BTreeMap<String, (String, String)>>,
    pub draft_versions: RwSignal<BTreeMap<String, u64>>,
    pub draft_clears: RwSignal<BTreeMap<String, u64>>,
    pub invalidated_reviews: RwSignal<BTreeSet<String>>,
    pub public_identities: RwSignal<BTreeMap<String, PublicIdentity>>,
    pub is_loading_feed: RwSignal<bool>,
    pub is_loading_wallet: RwSignal<bool>,
    pub status_message: RwSignal<Said>,
    pub synced: RwSignal<Synced>,
    pub publication: RwSignal<Option<Publication>>,
    pub following: RwSignal<bool>,
    pub pause_requested: RwSignal<bool>,
    pub publication_error: RwSignal<Said>,
    pub connection_health: RwSignal<Option<ConnectionHealth>>,
    pub connection_last_success_ms: RwSignal<Option<u64>>,
    pub is_checking_connection: RwSignal<bool>,
    pub speaking: RwSignal<Speaking>,
    pub topics: RwSignal<Remembered>,
    pub only_followed: RwSignal<bool>,
}

fn voiced(fault: Fault) -> Said {
    Said::Something(fault.message)
}

fn post_draft_key(content: &Content) -> Option<String> {
    let Content::Post { addressed, .. } = content else {
        return None;
    };
    Some(match addressed {
        Addressed::Standalone => String::new(),
        Addressed::Thread(txid) => txid.to_display_hex(),
    })
}

impl AppState {
    pub fn started() -> Self {
        let autoload = stored_autoload();
        let replies = stored_replies_shown();
        let recalled = topics::load();
        let state = Self::empty(autoload.value, replies.value, recalled.value);
        for complaint in [autoload.complaint, replies.complaint, recalled.complaint] {
            match complaint {
                Complaint::Unreadable(reason) => state.status_message.set(Said::Something(reason)),
                Complaint::Nothing => (),
            }
        }
        state.load_identity();
        let chain = state.active_chain.get_untracked();
        state.is_loading_feed.set(true);
        leptos::task::spawn_local(async move {
            sync_feed(state, chain).await;
            load_pending(state).await;
        });
        state
    }

    fn empty(autoload: bool, replies: usize, remembered: Remembered) -> Self {
        Self {
            active_chain: RwSignal::new(Chain::Litecoin),
            autoload_images: RwSignal::new(autoload),
            replies_shown: RwSignal::new(replies),
            media_epoch: RwSignal::new(0),
            media_inflight: RwSignal::new(0),
            media_bytes: RwSignal::new(0),
            wallet_summary: RwSignal::new(None),
            identity: RwSignal::new(None),
            identity_error: RwSignal::new(Said::Nothing),
            feed: RwSignal::new(Vec::new()),
            drafts: RwSignal::new(BTreeMap::new()),
            draft_photos: RwSignal::new(BTreeMap::new()),
            draft_labels: RwSignal::new(BTreeMap::new()),
            draft_versions: RwSignal::new(BTreeMap::new()),
            draft_clears: RwSignal::new(BTreeMap::new()),
            invalidated_reviews: RwSignal::new(BTreeSet::new()),
            public_identities: RwSignal::new(BTreeMap::new()),
            is_loading_feed: RwSignal::new(false),
            is_loading_wallet: RwSignal::new(false),
            status_message: RwSignal::new(Said::Nothing),
            synced: RwSignal::new(Synced::Unchecked),
            publication: RwSignal::new(None),
            following: RwSignal::new(false),
            pause_requested: RwSignal::new(false),
            publication_error: RwSignal::new(Said::Nothing),
            connection_health: RwSignal::new(None),
            connection_last_success_ms: RwSignal::new(None),
            is_checking_connection: RwSignal::new(false),
            speaking: RwSignal::new(Speaking::Freely),
            topics: RwSignal::new(remembered),
            only_followed: RwSignal::new(false),
        }
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
                Err(fault) => {
                    warn!(fault = %fault, "connection check could not complete");
                    state
                        .connection_health
                        .set(Some(unreachable(chain, fault)));
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
                    let unlocked = status.unlocked();
                    state.identity.set(Some(status));
                    state.identity_error.set(Said::Nothing);
                    match unlocked {
                        true => state.load_wallet(),
                        false => state.wallet_summary.set(None),
                    }
                }
                Err(fault) => {
                    warn!(fault = %fault, "identity status unavailable");
                    state.identity_error.set(voiced(fault));
                }
            }
        });
    }

    pub fn refresh_all(&self) {
        if self.is_loading_feed.get_untracked() {
            self.load_wallet();
            return;
        }
        let state = *self;
        let chain = state.active_chain.get_untracked();
        state.is_loading_feed.set(true);
        leptos::task::spawn_local(async move {
            sync_feed(state, chain).await;
            state.load_wallet();
        });
    }

    pub fn load_feed(&self) {
        if self.is_loading_feed.get_untracked() {
            return;
        }
        let state = *self;
        let chain = state.active_chain.get_untracked();
        state.is_loading_feed.set(true);
        leptos::task::spawn_local(sync_feed(state, chain));
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
                Err(fault) => {
                    warn!(fault = %fault, "funding summary unavailable");
                    state.wallet_summary.set(None);
                    state.status_message.set(voiced(fault));
                }
            }
            state.is_loading_wallet.set(false);
        });
    }

    pub fn follow(&self, id: String, approve: bool) {
        if self.following.get_untracked() {
            return;
        }
        if self.invalidated_reviews.get_untracked().contains(&id)
            || self.publication.get_untracked().as_ref().map(|held| &held.id) != Some(&id)
        {
            self.publication_error.set(Said::Something("This review changed. Review the current draft again before publishing.".into()));
            return;
        }
        let state = *self;
        let reviewed_draft = if approve {
            self.publication
                .get_untracked()
                .and_then(|review| post_draft_key(&review.content))
                .map(|key| {
                    let version = self.draft_version(&key);
                    (key, version)
                })
        } else {
            None
        };
        state.following.set(true);
        state.pause_requested.set(false);
        state.publication_error.set(Said::Nothing);
        leptos::task::spawn_local(async move {
            let mut approval = approve;
            while !state.pause_requested.get_untracked() {
                match publication::advance(id.clone(), approval).await {
                    Ok(result) => {
                        let complete = result.complete;
                        if !state.apply_advance_result(result, approval, reviewed_draft.as_ref()) {
                            break;
                        }
                        if complete {
                            state.refresh_all();
                            break;
                        }
                    }
                    Err(fault) => {
                        warn!(fault = %fault, "publication paused on a source failure");
                        state.publication_error.set(voiced(fault));
                        break;
                    }
                }
                approval = false;
                gloo_timers::future::TimeoutFuture::new(PUBLICATION_POLL_MS).await;
            }
            state.following.set(false);
        });
    }

    fn apply_advance_result(
        &self,
        result: Publication,
        approval: bool,
        reviewed_draft: Option<&(String, u64)>,
    ) -> bool {
        if !result.approved && self.invalidated_reviews.get_untracked().contains(&result.id) {
            return false;
        }
        if result.approved {
            self.accept_review(&result.id);
            if approval {
                if let Some((key, version)) = reviewed_draft {
                    if post_draft_key(&result.content).as_deref() == Some(key.as_str())
                        && self.draft_version(key) == *version
                    {
                        self.clear_draft(key);
                    }
                }
            }
        }
        self.publication.set(Some(result));
        true
    }

    pub fn reply_to(&self, post: Post) {
        self.speaking.set(Speaking::InReplyTo(post));
    }

    pub fn stop_replying(&self) {
        self.speaking.set(Speaking::Freely);
    }

    pub fn toggle_reply_to(&self, post: Post) {
        match self.speaking.with(|speaking| speaking.replies_to(post.txid)) {
            true => self.stop_replying(),
            false => self.reply_to(post),
        }
    }

    pub fn draft(&self, key: &str) -> String {
        self.drafts.with(|drafts| {
            let Some(held) = drafts.get(key) else {
                return String::new();
            };
            held.clone()
        })
    }

    pub fn draft_untracked(&self, key: &str) -> String {
        self.drafts.with_untracked(|drafts| {
            let Some(held) = drafts.get(key) else {
                return String::new();
            };
            held.clone()
        })
    }

    pub fn set_draft(&self, key: &str, text: String) {
        self.drafts.update(|drafts| {
            drafts.insert(key.to_owned(), text);
        });
        self.changed_draft(key);
    }

    pub fn photos(&self, key: &str) -> Vec<Photo> {
        self.draft_photos.with(|drafts| drafts.get(key).cloned().unwrap_or_default())
    }

    pub fn set_photos(&self, key: &str, photos: Vec<Photo>) {
        self.draft_photos.update(|drafts| {
            drafts.insert(key.to_owned(), photos);
        });
        self.changed_draft(key);
    }

    pub fn draft_labels(&self, key: &str) -> (String, String) {
        self.draft_labels.with(|drafts| drafts.get(key).cloned().unwrap_or_default())
    }

    pub fn set_draft_labels(&self, key: &str, wire: String, hashtags: String) {
        let held = self
            .draft_labels
            .with_untracked(|drafts| drafts.get(key).cloned().unwrap_or_default());
        if held == (wire.clone(), hashtags.clone()) {
            return;
        }
        self.draft_labels.update(|drafts| {
            if wire.is_empty() && hashtags.is_empty() {
                drafts.remove(key);
            } else {
                drafts.insert(key.to_owned(), (wire, hashtags));
            }
        });
        self.changed_draft(key);
    }

    pub fn draft_clear_revision(&self, key: &str) -> u64 {
        self.draft_clears.with(|clears| clears.get(key).copied().unwrap_or(0))
    }

    pub fn draft_version(&self, key: &str) -> u64 {
        self.draft_versions.with_untracked(|versions| versions.get(key).copied().unwrap_or(0))
    }

    pub fn accept_review(&self, id: &str) {
        self.invalidated_reviews.update(|old| { old.remove(id); });
    }

    fn changed_draft(&self, key: &str) {
        self.draft_versions.update(|versions| {
            let version = versions.entry(key.to_owned()).or_default();
            *version = version.wrapping_add(1);
        });
        let Some(review) = self.publication.get_untracked() else {
            return;
        };
        if review.approved || post_draft_key(&review.content).as_deref() != Some(key) {
            return;
        }
        self.invalidated_reviews.update(|old| { old.insert(review.id.clone()); });
        self.publication.set(None);
        let state = *self;
        leptos::task::spawn_local(async move {
            if let Err(fault) = publication::discard(review.id).await {
                warn!(fault = %fault, "old unapproved quote could not be discarded");
                state.publication_error.set(voiced(fault));
            }
        });
    }

    pub fn clear_draft(&self, key: &str) {
        self.drafts.update(|drafts| {
            drafts.remove(key);
        });
        self.draft_photos.update(|drafts| {
            drafts.remove(key);
        });
        self.draft_labels.update(|drafts| {
            drafts.remove(key);
        });
        self.draft_clears.update(|clears| {
            let revision = clears.entry(key.to_owned()).or_default();
            *revision = revision.wrapping_add(1);
        });
    }

    pub fn remember_topics(&self, wire: &str, hashtags: &[String]) {
        self.topics.update(|remembered| {
            remembered.used(wire, hashtags);
            topics::save(remembered);
        });
    }

    pub fn toggle_follow(&self, spelled: String) {
        self.topics.update(|remembered| {
            remembered.toggle(spelled);
            topics::save(remembered);
        });
    }

    pub fn seen_topics(&self) -> BTreeSet<String> {
        seen_in(&self.feed.get())
    }

    pub fn toggle_only_followed(&self) {
        self.only_followed.update(|only| *only = !*only);
    }

    pub fn conversations(&self) -> Vec<Conversation> {
        let preview = self.replies_shown.get();
        self.feed
            .with(|everything| conversations(self.shown(everything), everything, preview))
    }

    fn shown(&self, everything: &[Post]) -> Vec<Post> {
        if !self.only_followed.get() {
            return everything.to_vec();
        }
        let followed = self.topics.get().followed_labels();
        if followed.is_empty() {
            return everything.to_vec();
        }
        following(everything, &followed)
    }

    pub fn named(&self, author: AuthorId) -> Named {
        self.public_identities.with(|identities| {
            let Some(identity) = identities.get(&author.to_hex()) else {
                return Named::Unnamed;
            };
            let Some(nickname) = identity.nickname.as_ref() else {
                return Named::Unnamed;
            };
            match nickname.is_empty() {
                true => Named::Unnamed,
                false => Named::Nickname(nickname.clone()),
            }
        })
    }

    pub fn display_name(&self, author: AuthorId) -> String {
        match self.named(author) {
            Named::Nickname(nickname) => nickname,
            Named::Unnamed => author.fingerprint(),
        }
    }
}

pub enum Named {
    Nickname(String),
    Unnamed,
}

async fn sync_feed(state: AppState, chain: Chain) {
    match records::feed(chain).await {
        Ok(result) => {
            if state.active_chain.get_untracked() == chain {
                state.public_identities.set(result.identities);
                state.feed.set(result.posts);
                state.synced.set(Synced::Indexed {
                    start: result.start,
                    tip: result.tip,
                    complete: result.complete_to_tip,
                });
            }
        }
        Err(fault) => {
            warn!(fault = %fault, "feed sync failed; cached posts stand");
            state.synced.set(Synced::Failed(fault.message));
        }
    }
    state.is_loading_feed.set(false);
}

async fn load_pending(state: AppState) {
    match publication::pending().await {
        Ok(Pending::Awaiting(found)) => {
            if !state.invalidated_reviews.get_untracked().contains(&found.id) {
                state.publication.set(Some(found));
            }
        }
        Ok(Pending::Settled) => state.publication.set(None),
        Err(fault) => {
            warn!(fault = %fault, "pending publication unavailable");
            state.publication_error.set(voiced(fault));
        }
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
    match status.session {
        Session::Unlocked(author) => Signer::Author(author),
        Session::Locked => Signer::Locked,
    }
}

fn signed_by(state: AppState) -> Session {
    let Some(status) = state.identity.get_untracked() else {
        return Session::Locked;
    };
    status.session
}

#[derive(Clone, PartialEq, Eq)]
pub enum Synced {
    Unchecked,
    Indexed {
        start: u64,
        tip: u64,
        complete: bool,
    },
    Failed(String),
}

impl Synced {
    pub fn summary(&self) -> String {
        match self {
            Synced::Unchecked => String::new(),
            Synced::Indexed {
                start,
                tip,
                complete: true,
            } => format!("Blocks {start}–{tip} · up to date"),
            Synced::Indexed {
                start,
                tip,
                complete: false,
            } => format!("Blocks {start}–{tip} · more history remains"),
            Synced::Failed(_) => "Sync failed · cached records shown".to_owned(),
        }
    }

    pub fn trouble(&self) -> Said {
        match self {
            Synced::Failed(reason) => Said::Something(format!(
                "Sync failed; displayed records are cached: {reason}"
            )),
            Synced::Unchecked => Said::Nothing,
            Synced::Indexed { .. } => Said::Nothing,
        }
    }
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

fn unreachable(chain: Chain, fault: Fault) -> ConnectionHealth {
    ConnectionHealth {
        network: chain.network_name().to_owned(),
        source_type: "Unknown".to_owned(),
        endpoint: "Endpoint hidden".to_owned(),
        checked_at_ms: 0,
        remote_height: None,
        local_start: None,
        local_height: None,
        trouble: Trouble::SourceUnverified(fault.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::Owner;

    fn approved(id: &str) -> Publication {
        Publication {
            content: Content::Post {
                text: "Reviewed text".into(),
                wire: "original".into(),
                hashtags: vec![],
                addressed: Addressed::Standalone,
            },
            id: id.into(),
            txid: String::new(),
            fee: 0,
            selected_funding: 0,
            returned: 0,
            approved: true,
            confirmed: 0,
            mempool: 0,
            message: String::new(),
            complete: false,
        }
    }

    #[test]
    fn delayed_approval_keeps_new_edits_and_clears_only_the_unchanged_draft() {
        Owner::new().with(|| {
            let state = AppState::empty(false, 0, Remembered::default());
            state.set_draft("", "Reviewed text".into());
            state.set_photos(
                "",
                vec![Photo::new("https://cdn.example/old.png", "Old").unwrap()],
            );
            state.set_draft_labels("", "original".into(), "first".into());
            let reviewed = (String::new(), state.draft_version(""));

            state.set_draft("", "New text while approval was pending".into());
            let new_photo = Photo::new("https://cdn.example/new.png", "New").unwrap();
            state.set_photos("", vec![new_photo.clone()]);
            state.set_draft_labels("", "new-wire".into(), "second".into());
            state.invalidated_reviews.update(|ids| {
                ids.insert("old-plan".into());
            });
            assert!(state.apply_advance_result(approved("old-plan"), true, Some(&reviewed)));
            assert_eq!(state.draft(""), "New text while approval was pending");
            assert_eq!(state.photos(""), vec![new_photo]);
            assert_eq!(state.draft_labels(""), ("new-wire".into(), "second".into()));
            assert!(state.publication.get_untracked().unwrap().approved);
            assert!(!state.invalidated_reviews.get_untracked().contains("old-plan"));

            let clean = AppState::empty(false, 0, Remembered::default());
            clean.set_draft("", "Reviewed text".into());
            clean.set_photos(
                "",
                vec![Photo::new("https://cdn.example/old.png", "Old").unwrap()],
            );
            clean.set_draft_labels("", "original".into(), "first".into());
            let reviewed = (String::new(), clean.draft_version(""));
            assert!(clean.apply_advance_result(approved("clean-plan"), true, Some(&reviewed)));
            assert_eq!(clean.draft(""), "");
            assert!(clean.photos("").is_empty());
            assert_eq!(clean.draft_labels(""), (String::new(), String::new()));
            assert_eq!(clean.draft_clear_revision(""), 1);
        });
    }
}
