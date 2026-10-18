use crate::data::identity;
use crate::state::said::Said;
use crate::state::session::{AppState, Identity};
use leptos::prelude::expect_context;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use tracing::warn;
use wire_app_core::errors::Fault;
use wire_app_core::session::{Credential, IdentityStatus, Vault};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Create,
    Recover,
    Import,
    Unlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    ByStatus,
    Step(Step),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Reading,
    Unavailable(String),
    Entry(Step, IdentityStatus),
    Active(IdentityStatus),
}

pub fn screen(identity: &Identity, chosen: Chosen) -> Screen {
    let status = match identity {
        Identity::Reading => return Screen::Reading,
        Identity::Unavailable(reason) => return Screen::Unavailable(reason.clone()),
        Identity::Known(status) => status.clone(),
    };
    if status.unlocked() {
        return Screen::Active(status);
    }
    let step = match (chosen, &status.vault) {
        (Chosen::Step(step), _vault) => step,
        (Chosen::ByStatus, Vault::Missing) => Step::Welcome,
        (Chosen::ByStatus, Vault::Local | Vault::Configured { .. }) => Step::Unlock,
    };
    Screen::Entry(step, status)
}

pub fn replacing(status: &IdentityStatus) -> Said {
    match status.has_local_copy() {
        true => Said::Something(
            "This replaces the identity stored on this device. Keep its 12 words if you ever want it back."
                .to_owned(),
        ),
        false => Said::Nothing,
    }
}

pub fn verb(base: &'static str, status: &IdentityStatus) -> String {
    match status.has_local_copy() {
        true => format!("Replace and {}", base.to_lowercase()),
        false => base.to_owned(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Create,
    Recover,
    Import,
    Unlock,
    UnlockWithFile,
    Lock,
}

#[derive(Clone, Copy)]
pub struct Onboarding {
    pub chosen: RwSignal<Chosen>,
    pub password: RwSignal<String>,
    pub phrase: RwSignal<String>,
    pub revealed: RwSignal<Said>,
    pub busy: RwSignal<bool>,
    pub trouble: RwSignal<Said>,
    state: AppState,
}

impl Onboarding {
    pub fn opened() -> Self {
        Self::over(expect_context::<AppState>())
    }

    pub fn over(state: AppState) -> Self {
        Self {
            chosen: RwSignal::new(Chosen::ByStatus),
            password: RwSignal::new(String::new()),
            phrase: RwSignal::new(String::new()),
            revealed: RwSignal::new(Said::Nothing),
            busy: RwSignal::new(false),
            trouble: RwSignal::new(Said::Nothing),
            state,
        }
    }

    pub fn screen(&self) -> Screen {
        let identity = self.state.identity.get();
        screen(&identity, self.chosen.get())
    }

    pub fn go(&self, step: Step) {
        self.trouble.set(Said::Nothing);
        self.chosen.set(Chosen::Step(step));
    }

    pub fn back(&self) {
        self.trouble.set(Said::Nothing);
        self.chosen.set(Chosen::ByStatus);
    }

    pub fn dismiss_words(&self) {
        self.revealed.set(Said::Nothing);
    }

    fn replace(&self) -> bool {
        match self.state.identity.get_untracked() {
            Identity::Known(status) => status.has_local_copy(),
            Identity::Reading => false,
            Identity::Unavailable(_reason) => false,
        }
    }

    pub fn act(&self, intent: Intent) {
        if self.busy.get_untracked() {
            return;
        }
        let flow = *self;
        flow.busy.set(true);
        flow.trouble.set(Said::Nothing);
        let password = flow.password.get_untracked();
        flow.password.set(String::new());
        let phrase = flow.phrase.get_untracked();
        let replace = flow.replace();
        leptos::task::spawn_local(async move {
            match perform(intent, password, phrase, replace).await {
                Ok(Settled { status, words }) => {
                    flow.phrase.set(String::new());
                    flow.chosen.set(Chosen::ByStatus);
                    flow.revealed.set(words);
                    flow.state.identity_is(status);
                }
                Err(fault) => {
                    warn!(fault = %fault, intent = ?intent, "identity action refused");
                    flow.trouble.set(Said::Something(fault.message));
                }
            }
            flow.busy.set(false);
        });
    }
}

struct Settled {
    status: IdentityStatus,
    words: Said,
}

async fn perform(
    intent: Intent,
    password: String,
    phrase: String,
    replace: bool,
) -> Result<Settled, Fault> {
    let status = match intent {
        Intent::Create => {
            let created = identity::create(password, replace).await?;
            return Ok(Settled {
                status: created.status,
                words: Said::Something(created.phrase),
            });
        }
        Intent::Recover => identity::recover(phrase, password, replace).await?,
        Intent::Import => {
            let bytes = identity::selected_vault().await?;
            identity::import(bytes, password, replace).await?
        }
        Intent::Unlock => identity::unlock(Credential::Password(password)).await?,
        Intent::UnlockWithFile => identity::unlock(Credential::ConfiguredFile).await?,
        Intent::Lock => identity::lock().await?,
    };
    Ok(Settled {
        status,
        words: Said::Nothing,
    })
}
