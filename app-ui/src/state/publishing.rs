use crate::config::PUBLICATION_POLL_MS;
use crate::data::publication;
use crate::state::said::Said;
use crate::state::session::{voiced, AppState};
use leptos::prelude::GetUntracked;
use leptos::prelude::Set;
use leptos::prelude::WithUntracked;
use tracing::warn;
use wire_app_core::publication::{Addressed, Content, Pending, Publication};

pub enum Reviewed {
    Draft { key: String, version: u64 },
    Nothing,
}

pub fn draft_key(addressed: &Addressed) -> String {
    match addressed {
        Addressed::Standalone => String::new(),
        Addressed::Thread(txid) => txid.to_display_hex(),
    }
}

impl AppState {
    pub fn follow(&self, id: String, approve: bool) {
        if self.following.get_untracked() {
            return;
        }
        if self.invalidated_reviews.get_untracked().contains(&id) || !self.holds(&id) {
            self.publication_error.set(Said::Something(
                "This review changed. Review the current draft again before publishing.".into(),
            ));
            return;
        }
        let state = *self;
        let reviewed = match approve {
            true => self.reviewed(),
            false => Reviewed::Nothing,
        };
        state.following.set(true);
        state.publication_error.set(Said::Nothing);
        leptos::task::spawn_local(async move {
            let mut approval = approve;
            loop {
                match publication::advance(id.clone(), approval).await {
                    Ok(result) => {
                        let complete = result.complete;
                        if !state.apply_advance_result(result, approval, &reviewed) {
                            break;
                        }
                        if complete {
                            state.refresh_all();
                            break;
                        }
                    }
                    Err(fault) => {
                        if state.holds(&id) {
                            warn!(fault = %fault, "publication paused on a source failure");
                            state.publication_error.set(voiced(fault));
                        }
                        break;
                    }
                }
                approval = false;
                gloo_timers::future::TimeoutFuture::new(PUBLICATION_POLL_MS).await;
            }
            state.following.set(false);
        });
    }

    pub fn holds(&self, id: &str) -> bool {
        self.publication.with_untracked(|held| {
            let Some(held) = held else {
                return false;
            };
            held.id == id
        })
    }

    fn reviewed(&self) -> Reviewed {
        self.publication.with_untracked(|held| {
            let Some(review) = held else {
                return Reviewed::Nothing;
            };
            let Content::Post { addressed, .. } = &review.content else {
                return Reviewed::Nothing;
            };
            let key = draft_key(addressed);
            Reviewed::Draft {
                version: self.draft_version(&key),
                key,
            }
        })
    }

    pub fn apply_advance_result(
        &self,
        result: Publication,
        approval: bool,
        reviewed: &Reviewed,
    ) -> bool {
        if !self.holds(&result.id) {
            return false;
        }
        if !result.approved
            && self
                .invalidated_reviews
                .get_untracked()
                .contains(&result.id)
        {
            return false;
        }
        if result.approved {
            self.accept_review(&result.id);
            if approval {
                self.clear_reviewed(&result.content, reviewed);
            }
        }
        self.publication.set(Some(result));
        true
    }

    fn clear_reviewed(&self, content: &Content, reviewed: &Reviewed) {
        let Reviewed::Draft { key, version } = reviewed else {
            return;
        };
        let Content::Post { addressed, .. } = content else {
            return;
        };
        if draft_key(addressed) == *key && self.draft_version(key) == *version {
            self.clear_draft(key);
        }
    }
}

pub async fn load_pending(state: AppState) {
    match publication::pending().await {
        Ok(Pending::Awaiting(found)) => {
            if found.complete
                || state
                    .invalidated_reviews
                    .get_untracked()
                    .contains(&found.id)
            {
                return;
            }
            let resume = found.approved;
            let id = found.id.clone();
            state.publication.set(Some(found));
            if resume {
                state.follow(id, false);
            }
        }
        Ok(Pending::Settled) => state.publication.set(None),
        Err(fault) => {
            warn!(fault = %fault, "pending publication unavailable");
            state.publication_error.set(voiced(fault));
        }
    }
}
