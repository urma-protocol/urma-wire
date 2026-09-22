use crate::components::Said;
use crate::data::identity;
use crate::data::identity::Unlock;
use crate::state::session::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::GetUntracked;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Update;
use tracing::warn;

#[derive(Clone, Copy)]
pub struct Vault {
    pub password: RwSignal<String>,
    pub busy: RwSignal<bool>,
    pub message: RwSignal<Said>,
    state: AppState,
}

#[derive(Clone, Copy)]
pub enum Intent {
    Unlock,
    UseConfiguredFile,
    Import,
    Lock,
}

impl Vault {
    pub fn opened() -> Self {
        Self {
            password: RwSignal::new(String::new()),
            busy: RwSignal::new(false),
            message: RwSignal::new(Said::Nothing),
            state: expect_context::<AppState>(),
        }
    }

    pub fn act(&self, intent: Intent) {
        if self.busy.get_untracked() {
            return;
        }
        let state = self.state;
        let vault = *self;
        vault.busy.set(true);
        vault.message.set(Said::Nothing);
        let credential = vault.password.get_untracked();
        vault.password.set(String::new());
        leptos::task::spawn_local(async move {
            match perform(state, intent, credential).await {
                Outcome::Changed(spoken) => {
                    state.wallet_summary.set(None);
                    state.load_identity();
                    vault.message.set(Said::Something(spoken));
                }
                Outcome::Refused(reason) => vault.message.set(Said::Something(reason)),
            }
            vault.busy.set(false);
        });
    }
}

pub enum Outcome {
    Changed(String),
    Refused(String),
}

async fn perform(state: AppState, intent: Intent, credential: String) -> Outcome {
    match intent {
        Intent::Import => import_vault(state, credential).await,
        Intent::Lock => lock_vault(state).await,
        Intent::Unlock => open(Unlock::Password(credential)).await,
        Intent::UseConfiguredFile => open(Unlock::ConfiguredFile).await,
    }
}

async fn import_vault(state: AppState, password: String) -> Outcome {
    let bytes = match identity::selected_vault().await {
        Ok(bytes) => bytes,
        Err(failure) => {
            warn!(failure = %failure, "vault file could not be read");
            return Outcome::Refused(failure.reported());
        }
    };
    match identity::import(bytes, password).await {
        Ok(status) => {
            state.identity.set(Some(status));
            Outcome::Changed("Identity unlocked for this session.".to_owned())
        }
        Err(failure) => {
            warn!(failure = %failure, "vault import refused");
            Outcome::Refused(failure.reported())
        }
    }
}

async fn lock_vault(state: AppState) -> Outcome {
    match identity::lock().await {
        Ok(()) => {
            state.identity.update(forget_author);
            Outcome::Changed(
                "Identity locked. Already approved signed plans remain resumable.".to_owned(),
            )
        }
        Err(failure) => {
            warn!(failure = %failure, "lock refused");
            Outcome::Refused(failure.reported())
        }
    }
}

async fn open(credential: Unlock) -> Outcome {
    match identity::unlock(credential).await {
        Ok(()) => Outcome::Changed("Identity unlocked for this session.".to_owned()),
        Err(failure) => {
            warn!(failure = %failure, "unlock refused");
            Outcome::Refused(failure.reported())
        }
    }
}

fn forget_author(status: &mut Option<wire_app_shared::IdentityStatus>) {
    let Some(status) = status.as_mut() else {
        return;
    };
    status.unlocked = false;
    status.author_pubkey = None;
    status.author_fingerprint = None;
}
