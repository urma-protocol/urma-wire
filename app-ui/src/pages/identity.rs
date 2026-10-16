use crate::components::profile_editor::ProfileEditor;
use crate::components::Actions;
use crate::components::DescribedGroup;
use crate::components::Fact;
use crate::components::Facts;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::components::Status;
use crate::components::Tone;
use crate::config::{PASSWORD_RULE, VAULT_INPUT_ID};
use crate::state::vault::Intent;
use crate::state::vault::Vault;
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
use leptos::prelude::{component, event_target_value, view};
use wire_app_core::session::{IdentityStatus, Session, VaultSource};

#[component]
pub fn IdentityPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let vault = Vault::opened();
    let busy = move || vault.busy.get();

    view! {
        <section class="page-container settings-page">
            <PageHeader eyebrow="YOUR KEYS. YOUR CONTROL." title="Your" accent="identity.">
                "Unlock once to sign during this session. Restarting Wire locks your identity. Funding and network availability are shown in Wallet."
            </PageHeader>
            <Notice message=state.identity_error.into() />
            <Notice message=vault.message.into() />

            <SettingsGroup title="Active identity">
                {move || match state.identity.get() {
                    Some(status) => view! { <ActiveIdentity status=status /> }.into_any(),
                    None => view! { <p class="loading-state" role="status">"Reading session status…"</p> }.into_any(),
                }}
                <Actions>
                    {move || match profile_link(state) {
                        Linked::Profile(href) => view! {
                            <a class="btn btn-secondary btn-sm" href=href>"View public profile"</a>
                        }.into_any(),
                        Linked::Unlinked => ().into_any(),
                    }}
                    <button class="btn btn-secondary btn-sm" disabled=busy
                        on:click=move |_clicked| state.load_identity()>"Refresh session status"</button>
                </Actions>
            </SettingsGroup>

            <UnlockGroup vault=vault />

            <ProfileEditor />
        </section>
    }
}

#[component]
fn UnlockGroup(vault: Vault) -> impl IntoView {
    let state = expect_context::<AppState>();
    let busy = move || vault.busy.get();
    let act = move |intent: Intent| move |_clicked| vault.act(intent);
    let unlock_on_enter = move |event: leptos::ev::KeyboardEvent| {
        if event.key() == "Enter" {
            vault.act(Intent::Unlock);
        }
    };
    view! {
        <DescribedGroup
            title="Unlock, create or import"
            description="Your password decrypts the vault for this session only; Wire never stores it. Reading the feed needs no identity."
        >
            {move || match configured_unlock(state) {
                Unlocking::ConfiguredFile(path) => view! {
                    <Actions>
                        <button class="btn btn-primary" disabled=busy
                            on:click=move |_clicked| vault.act(Intent::UseConfiguredFile)>
                            "Use configured unlock file"
                        </button>
                    </Actions>
                    <p class="group-desc">"Reads the unlock secret from " <code class="code-val">{path}</code> "; nothing is shown or stored."</p>
                }.into_any(),
                Unlocking::PasswordOnly => ().into_any(),
            }}
            <label class="field">"Vault password"
                <input type="password" autocomplete="current-password"
                    prop:value=move || vault.password.get()
                    on:input=move |event| vault.password.set(event_target_value(&event))
                    on:keydown=unlock_on_enter />
                <span class="field-hint">{PASSWORD_RULE}</span>
            </label>
            <Actions>
                {move || match configured_unlock(state) {
                    Unlocking::ConfiguredFile(_path) => view! {
                        <button class="btn btn-secondary" disabled=busy on:click=act(Intent::Unlock)>"Unlock with password"</button>
                    }.into_any(),
                    Unlocking::PasswordOnly => view! {
                        <button class="btn btn-primary" disabled=busy on:click=act(Intent::Unlock)>"Unlock identity"</button>
                    }.into_any(),
                }}
                <button class="btn btn-secondary" disabled=move || busy() || !unlocked(state)
                    on:click=act(Intent::Lock)>"Lock identity"</button>
            </Actions>
            <PhraseActions vault=vault />
            <ImportActions vault=vault />
        </DescribedGroup>
    }
}

#[component]
fn PhraseActions(vault: Vault) -> impl IntoView {
    let state = expect_context::<AppState>();
    let blocked = move || vault.busy.get() || already_imported(state);
    let act = move |intent: Intent| move |_clicked| vault.act(intent);
    view! {
        <p class="group-desc">
            "Or start with 12 words: type the words you already have to recover that identity, or generate new ones. The vault password above protects the copy this app keeps; the words are the only way to recover the identity elsewhere."
        </p>
        <label class="field">"12 words"
            <textarea rows="2" autocomplete="off" spellcheck="false"
                prop:value=move || vault.phrase.get()
                on:input=move |event| vault.phrase.set(event_target_value(&event)) />
        </label>
        <Actions>
            <button class="btn btn-primary"
                disabled=move || blocked() || vault.phrase.get().trim().is_empty()
                on:click=act(Intent::Create)>"Recover from 12 words"</button>
            <button class="btn btn-secondary" disabled=blocked
                on:click=act(Intent::Generate)>"Generate a new identity"</button>
        </Actions>
        {move || match vault.revealed.get() {
            Said::Something(words) => view! {
                <div class="phrase-reveal" role="status">
                    <p class="group-desc">"Your 12 words. Write them down now; they are shown only once."</p>
                    <code class="code-val">{words}</code>
                </div>
            }.into_any(),
            Said::Nothing => ().into_any(),
        }}
    }
}

#[component]
fn ImportActions(vault: Vault) -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <p class="group-desc">
            "Or import a copy of an existing encrypted URMA vault with its password. Import validates the password and unlocks its active identity. Your original vault stays intact, and an existing imported vault is never overwritten."
        </p>
        <label class="field">"Encrypted URMA vault"
            <input id=VAULT_INPUT_ID type="file" accept=".urma,.vault" />
        </label>
        <Actions>
            <button class="btn btn-primary"
                disabled=move || vault.busy.get() || already_imported(state)
                on:click=move |_clicked| vault.act(Intent::Import)>"Import & unlock vault"</button>
        </Actions>
    }
}

enum Unlocking {
    ConfiguredFile(String),
    PasswordOnly,
}

fn configured_unlock(state: AppState) -> Unlocking {
    let Some(status) = state.identity.get() else {
        return Unlocking::PasswordOnly;
    };
    match status.configured_unlock_available {
        true => Unlocking::ConfiguredFile(status.configured_unlock_path),
        false => Unlocking::PasswordOnly,
    }
}

fn already_imported(state: AppState) -> bool {
    let Some(status) = state.identity.get() else {
        return false;
    };
    status.source == VaultSource::Imported
}

fn unlocked(state: AppState) -> bool {
    let Some(status) = state.identity.get() else {
        return false;
    };
    status.unlocked()
}

enum Linked {
    Profile(String),
    Unlinked,
}

fn profile_link(state: AppState) -> Linked {
    let Some(status) = state.identity.get() else {
        return Linked::Unlinked;
    };
    match status.session {
        Session::Unlocked(author) => Linked::Profile(format!("/profile/{}", author.to_hex())),
        Session::Locked => Linked::Unlinked,
    }
}

#[component]
fn ActiveIdentity(status: IdentityStatus) -> impl IntoView {
    let (tone, session) = match status.session {
        Session::Unlocked(_author) => (Tone::Live, "Unlocked for this session"),
        Session::Locked => (Tone::Idle, "Locked"),
    };
    let origin = match status.source {
        VaultSource::Imported => "Using your imported encrypted vault.",
        VaultSource::Configured => "Using your configured URMA vault.",
        VaultSource::Missing => "Import an encrypted URMA vault to begin.",
    };
    view! {
        <div class="state-line">
            <Status tone=tone text=session />
            <p class="group-desc">{origin}</p>
        </div>
        {match status.session {
            Session::Unlocked(author) => view! {
                <Facts>
                    <Fact label="Fingerprint"><span class="badge badge-neutral">{author.fingerprint()}</span></Fact>
                    <Fact label="Author public key" wide=true><code class="code-val">{author.to_hex()}</code></Fact>
                </Facts>
            }.into_any(),
            Session::Locked => ().into_any(),
        }}
    }
}
