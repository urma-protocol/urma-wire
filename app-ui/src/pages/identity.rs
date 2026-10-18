use crate::components::profile_editor::ProfileEditor;
use crate::components::Actions;
use crate::components::Fact;
use crate::components::Facts;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::components::Status;
use crate::components::Tone;
use crate::components::Warning;
use crate::config::{PASSWORD_RULE, VAULT_INPUT_ID};
use crate::state::onboarding::{replacing, verb, Intent, Onboarding, Screen, Step};
use crate::state::AppState;
use crate::state::Said;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::Set;
use leptos::prelude::Signal;
use leptos::prelude::Children;
use leptos::prelude::{component, event_target_value, view};
use wire_app_core::session::{IdentityStatus, Session, Vault};

#[component]
pub fn IdentityPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let flow = Onboarding::opened();
    view! {
        <section class="page-container settings-page">
            <PageHeader eyebrow="YOUR KEYS. YOUR WORDS." title="Your" accent="identity.">
                "Twelve words are your identity. A password only protects the copy kept on this device. Reading the feed needs neither."
            </PageHeader>
            {move || match flow.screen() {
                Screen::Reading => view! { <p class="loading-state" role="status">"Reading session status…"</p> }.into_any(),
                Screen::Unavailable(reason) => view! {
                    <SettingsGroup title="Identity unavailable">
                        <p class="group-desc">{reason}</p>
                        <Actions>
                            <button class="btn btn-secondary" on:click=move |_clicked| state.load_identity()>"Retry"</button>
                        </Actions>
                    </SettingsGroup>
                }.into_any(),
                Screen::Entry(Step::Welcome, _status) => view! { <Welcome flow=flow /> }.into_any(),
                Screen::Entry(Step::Unlock, status) => view! { <UnlockScreen flow=flow status=status /> }.into_any(),
                Screen::Entry(Step::Create, status) => view! { <CreateStep flow=flow status=status /> }.into_any(),
                Screen::Entry(Step::Recover, status) => view! { <RecoverStep flow=flow status=status /> }.into_any(),
                Screen::Entry(Step::Import, status) => view! { <ImportStep flow=flow status=status /> }.into_any(),
                Screen::Active(status) => view! {
                    <ActiveIdentity flow=flow status=status />
                    <ProfileEditor />
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn Welcome(flow: Onboarding) -> impl IntoView {
    let go = move |step: Step| move |_clicked| flow.go(step);
    view! {
        <SettingsGroup title="Start here">
            <p class="group-desc">"Pick one. Each takes under a minute."</p>
            <div class="choice-grid">
                <button class="choice-card" on:click=go(Step::Create)>
                    <strong>"Create a new identity"</strong>
                    <span>"Wire draws 12 words for you. Write them down; they are the only backup."</span>
                </button>
                <button class="choice-card" on:click=go(Step::Recover)>
                    <strong>"I have 12 words"</strong>
                    <span>"Rebuild your identity from its words. No old password needed."</span>
                </button>
                <button class="choice-card" on:click=go(Step::Import)>
                    <strong>"Import a vault file"</strong>
                    <span>"Bring an encrypted URMA vault and its password."</span>
                </button>
            </div>
            <Warning message=flow.trouble.into() />
        </SettingsGroup>
    }
}

#[component]
fn UnlockScreen(flow: Onboarding, status: IdentityStatus) -> impl IntoView {
    let busy = move || flow.busy.get();
    let on_enter = move |event: leptos::ev::KeyboardEvent| {
        if event.key() == "Enter" {
            flow.act(Intent::Unlock);
        }
    };
    let (held, file) = match &status.vault {
        Vault::Local => ("An identity is stored on this device.".to_owned(), None),
        Vault::Configured { path, unlock_file } => {
            (format!("Using the URMA vault at {path}."), unlock_file.clone())
        }
        Vault::Missing => (String::new(), None),
    };
    view! {
        <SettingsGroup title="Unlock">
            <p class="group-desc">{held}" Unlocking lasts until you lock or close Wire."</p>
            <PasswordField flow=flow label="Device password" on_enter=on_enter />
            <Actions>
                <button class="btn btn-primary" disabled=busy on:click=move |_clicked| flow.act(Intent::Unlock)>
                    {move || match busy() { true => "Unlocking…", false => "Unlock" }}
                </button>
                {file.map(|path| view! {
                    <button class="btn btn-secondary" disabled=busy title=path
                        on:click=move |_clicked| flow.act(Intent::UnlockWithFile)>"Use unlock file"</button>
                })}
            </Actions>
            <Warning message=flow.trouble.into() />
            <p class="group-desc">
                <button class="link-button" on:click=move |_clicked| flow.go(Step::Recover)>"Forgot the password? Recover with the 12 words."</button>
                " · "
                <button class="link-button" on:click=move |_clicked| flow.go(Step::Welcome)>"Use a different identity"</button>
            </p>
        </SettingsGroup>
    }
}

#[component]
fn PasswordField(
    flow: Onboarding,
    label: &'static str,
    on_enter: impl Fn(leptos::ev::KeyboardEvent) + 'static,
) -> impl IntoView {
    view! {
        <label class="field">{label}
            <input type="password" autocomplete="off"
                prop:value=move || flow.password.get()
                on:input=move |event| flow.password.set(event_target_value(&event))
                on:keydown=on_enter />
            <span class="field-hint">{PASSWORD_RULE}</span>
        </label>
    }
}

#[component]
fn EntryStep(
    flow: Onboarding,
    title: &'static str,
    status: IdentityStatus,
    children: Children,
) -> impl IntoView {
    let warning = replacing(&status);
    view! {
        <SettingsGroup title=title>
            <Warning message=Signal::stored(warning) />
            {children()}
            <Warning message=flow.trouble.into() />
            <p class="group-desc">
                <button class="link-button" on:click=move |_clicked| flow.back()>"← Back"</button>
            </p>
        </SettingsGroup>
    }
}

#[component]
fn CreateStep(flow: Onboarding, status: IdentityStatus) -> impl IntoView {
    let busy = move || flow.busy.get();
    let label = verb("Create identity", &status);
    view! {
        <EntryStep flow=flow title="Create a new identity" status=status>
            <p class="group-desc">"Choose a password for the copy kept on this device. The 12 words appear once after creation."</p>
            <PasswordField flow=flow label="Device password" on_enter=move |event| {
                if event.key() == "Enter" { flow.act(Intent::Create); }
            } />
            <Actions>
                <button class="btn btn-primary" disabled=busy on:click=move |_clicked| flow.act(Intent::Create)>
                    {move || match busy() { true => "Creating…".to_owned(), false => label.clone() }}
                </button>
            </Actions>
        </EntryStep>
    }
}

#[component]
fn RecoverStep(flow: Onboarding, status: IdentityStatus) -> impl IntoView {
    let busy = move || flow.busy.get();
    let label = verb("Recover", &status);
    let ready = move || !flow.phrase.get().trim().is_empty();
    view! {
        <EntryStep flow=flow title="Recover from 12 words" status=status>
            <p class="group-desc">"Type the 12 words, then choose a new password for this device. The previous password is not needed."</p>
            <label class="field">"12 words"
                <textarea rows="2" autocomplete="off" spellcheck="false"
                    prop:value=move || flow.phrase.get()
                    on:input=move |event| flow.phrase.set(event_target_value(&event)) />
            </label>
            <PasswordField flow=flow label="New device password" on_enter=move |event| {
                if event.key() == "Enter" { flow.act(Intent::Recover); }
            } />
            <Actions>
                <button class="btn btn-primary" disabled=move || busy() || !ready()
                    on:click=move |_clicked| flow.act(Intent::Recover)>
                    {move || match busy() { true => "Recovering…".to_owned(), false => label.clone() }}
                </button>
            </Actions>
        </EntryStep>
    }
}

#[component]
fn ImportStep(flow: Onboarding, status: IdentityStatus) -> impl IntoView {
    let busy = move || flow.busy.get();
    let label = verb("Import", &status);
    view! {
        <EntryStep flow=flow title="Import a vault file" status=status>
            <p class="group-desc">"Wire checks the password, keeps an encrypted copy on this device and unlocks it. Your original file is untouched."</p>
            <label class="field">"Encrypted URMA vault"
                <input id=VAULT_INPUT_ID type="file" accept=".urma,.vault" />
            </label>
            <PasswordField flow=flow label="Vault password" on_enter=move |event| {
                if event.key() == "Enter" { flow.act(Intent::Import); }
            } />
            <Actions>
                <button class="btn btn-primary" disabled=busy on:click=move |_clicked| flow.act(Intent::Import)>
                    {move || match busy() { true => "Importing…".to_owned(), false => label.clone() }}
                </button>
            </Actions>
        </EntryStep>
    }
}

#[component]
fn ActiveIdentity(flow: Onboarding, status: IdentityStatus) -> impl IntoView {
    let busy = move || flow.busy.get();
    let Session::Unlocked(author) = status.session else {
        return ().into_any();
    };
    let kept = match &status.vault {
        Vault::Local => "Encrypted copy on this device.".to_owned(),
        Vault::Configured { path, .. } => format!("Encrypted copy at {path}."),
        Vault::Missing => "No encrypted copy; recover from the 12 words next time.".to_owned(),
    };
    view! {
        <SettingsGroup title="Active identity">
            <WordsOnce flow=flow />
            <div class="state-line">
                <Status tone=Tone::Live text="Unlocked" />
                <p class="group-desc">{kept}</p>
            </div>
            <Facts>
                <Fact label="Fingerprint"><span class="badge badge-neutral">{author.fingerprint()}</span></Fact>
                <Fact label="Author public key" wide=true><code class="code-val">{author.to_hex()}</code></Fact>
            </Facts>
            <Actions>
                <a class="btn btn-secondary btn-sm" href=format!("/profile/{}", author.to_hex())>"View public profile"</a>
                <button class="btn btn-secondary btn-sm" disabled=busy on:click=move |_clicked| flow.act(Intent::Lock)>"Lock"</button>
            </Actions>
            <Warning message=flow.trouble.into() />
        </SettingsGroup>
    }
    .into_any()
}

#[component]
fn WordsOnce(flow: Onboarding) -> impl IntoView {
    view! {
        {move || match flow.revealed.get() {
            Said::Something(words) => view! {
                <div class="phrase-reveal" role="status">
                    <p class="group-desc">"Your 12 words. Write them down now; Wire shows them only once and cannot show them again."</p>
                    <code class="code-val">{words}</code>
                    <Actions>
                        <button class="btn btn-primary btn-sm" on:click=move |_clicked| flow.dismiss_words()>"I wrote them down"</button>
                    </Actions>
                </div>
            }.into_any(),
            Said::Nothing => ().into_any(),
        }}
    }
}
