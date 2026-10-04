use crate::data::publication;
use crate::state::said::Said;
use crate::state::session::{voiced, AppState};
use leptos::prelude::GetUntracked;
use leptos::prelude::Set;
use tracing::warn;

impl AppState {
    pub fn abandon_publication(&self, id: String) {
        if self.following.get_untracked() {
            return;
        }
        let state = *self;
        leptos::task::spawn_local(async move {
            match publication::discard(id).await {
                Ok(()) => {
                    state.publication.set(None);
                    state.publication_error.set(Said::Nothing);
                    state.status_message.set(Said::Something(
                        "Publication abandoned. Its funding stays yours; review a new draft when ready."
                            .to_owned(),
                    ));
                    state.load_wallet();
                }
                Err(fault) => {
                    warn!(fault = %fault, "publication could not be abandoned");
                    state.publication_error.set(voiced(fault));
                }
            }
        });
    }
}
