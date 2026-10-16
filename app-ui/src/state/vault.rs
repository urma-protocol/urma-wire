use crate::data::identity;
use crate::state::said::Said;
use crate::state::session::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::GetUntracked;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use tracing::warn;
use wire_app_core::session::Credential;

#[derive(Clone, Copy)]
pub struct Vault {
    pub password: RwSignal<String>,
    pub phrase: RwSignal<String>,
    pub revealed: RwSignal<Said>,
    pub busy: RwSignal<bool>,
    pub message: RwSignal<Said>,
    state: AppState,
}

#[derive(Clone, Copy)]
pub enum Intent {
    Unlock,
    UseConfiguredFile,
    Import,
    Create,
    Generate,
    Lock,
}

impl Vault {
    pub fn opened() -> Self {
        Self {
            password: RwSignal::new(String::new()),
            phrase: RwSignal::new(String::new()),
            revealed: RwSignal::new(Said::Nothing),
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
        let words = match intent {
            Intent::Create => vault.phrase.get_untracked(),
            Intent::Generate
            | Intent::Unlock
            | Intent::UseConfiguredFile
            | Intent::Import
            | Intent::Lock => String::new(),
        };
        leptos::task::spawn_local(async move {
            match perform(state, intent, credential, words).await {
                Outcome::Changed(spoken) => {
                    vault.phrase.set(String::new());
                    state.wallet_summary.set(None);
                    state.load_identity();
                    vault.message.set(Said::Something(spoken));
                }
                Outcome::Revealed(phrase) => {
                    state.wallet_summary.set(None);
                    state.load_identity();
                    vault.revealed.set(Said::Something(phrase));
                    vault.message.set(Said::Something(
                        "New identity created and unlocked. Write the 12 words down now; they are the only way to recover it.".to_owned(),
                    ));
                }
                Outcome::Refused(reason) => vault.message.set(Said::Something(reason)),
            }
            vault.busy.set(false);
        });
    }
}

pub enum Outcome {
    Changed(String),
    Revealed(String),
    Refused(String),
}

async fn perform(state: AppState, intent: Intent, credential: String, words: String) -> Outcome {
    match intent {
        Intent::Import => import_vault(state, credential).await,
        Intent::Create => create_identity(state, words, credential).await,
        Intent::Generate => create_identity(state, String::new(), credential).await,
        Intent::Lock => lock_vault(state).await,
        Intent::Unlock => open(Credential::Password(credential)).await,
        Intent::UseConfiguredFile => open(Credential::ConfiguredFile).await,
    }
}

async fn create_identity(state: AppState, words: String, password: String) -> Outcome {
    if words.trim().is_empty() {
        return match identity::create(String::new(), password).await {
            Ok(created) => {
                state.identity.set(Some(created.status));
                Outcome::Revealed(created.phrase)
            }
            Err(fault) => {
                warn!(fault = %fault, "identity generation refused");
                Outcome::Refused(fault.message)
            }
        };
    }
    match identity::create(words, password).await {
        Ok(created) => {
            state.identity.set(Some(created.status));
            Outcome::Changed("Identity recovered from its 12 words and unlocked for this session.".to_owned())
        }
        Err(fault) => {
            warn!(fault = %fault, "identity recovery refused");
            Outcome::Refused(fault.message)
        }
    }
}

async fn import_vault(state: AppState, password: String) -> Outcome {
    let bytes = match identity::selected_vault().await {
        Ok(bytes) => bytes,
        Err(fault) => {
            warn!(fault = %fault, "vault file could not be read");
            return Outcome::Refused(fault.message);
        }
    };
    match identity::import(bytes, password).await {
        Ok(status) => {
            state.identity.set(Some(status));
            Outcome::Changed("Identity unlocked for this session.".to_owned())
        }
        Err(fault) => {
            warn!(fault = %fault, "vault import refused");
            Outcome::Refused(fault.message)
        }
    }
}

async fn lock_vault(state: AppState) -> Outcome {
    match identity::lock().await {
        Ok(()) => {
            state.identity_locked();
            Outcome::Changed(
                "Identity locked. Already approved signed plans remain resumable.".to_owned(),
            )
        }
        Err(fault) => {
            warn!(fault = %fault, "lock refused");
            Outcome::Refused(fault.message)
        }
    }
}

async fn open(credential: Credential) -> Outcome {
    match identity::unlock(credential).await {
        Ok(()) => Outcome::Changed("Identity unlocked for this session.".to_owned()),
        Err(fault) => {
            warn!(fault = %fault, "unlock refused");
            Outcome::Refused(fault.message)
        }
    }
}
