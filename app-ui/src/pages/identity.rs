use crate::components::profile_editor::ProfileEditor;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::limits::VAULT_INPUT_ID;
use crate::state::vault::Intent;
use crate::state::vault::Vault;
use crate::state::AppState;
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
use leptos::prelude::Show;
use leptos::prelude::{component, event_target_value, view};
use wire_app_shared::IdentityStatus;

#[component]
pub fn IdentityPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let vault = Vault::opened();
    let busy = move || vault.busy.get();
    let act = move |intent: Intent| move |_clicked| vault.act(intent);

    view! {
        <section class="page-container settings-page">
            <PageHeader
                eyebrow="YOUR KEYS. YOUR CONTROL."
                title="Your"
                accent="identity."
                subtitle="Unlock once to sign during this session. Restarting Wire locks your identity. Funding and network availability are shown in Wallet."
            />
            <Notice message=state.identity_error.into() />
            <Notice message=vault.message.into() />

            <SettingsGroup title="Active identity">
                {move || match state.identity.get() {
                    Some(status) => view! { <ActiveIdentity status=status /> }.into_any(),
                    None => ().into_any(),
                }}
                <button class="btn btn-secondary" disabled=busy
                    on:click=move |_| state.load_identity()>"Refresh session status"</button>
            </SettingsGroup>

            <SettingsGroup title="Unlock or import">
                <label>"Vault password"
                    <input type="password" autocomplete="off"
                        prop:value=move || vault.password.get()
                        on:input=move |event| vault.password.set(event_target_value(&event)) />
                </label>
                <button class="btn btn-primary" disabled=busy on:click=act(Intent::Unlock)>"Unlock identity"</button>
                <Show when=move || configured_unlock(state)>
                    <button class="btn btn-secondary" disabled=busy on:click=act(Intent::UseConfiguredFile)>"Use configured unlock file"</button>
                </Show>
                <button class="btn btn-secondary" disabled=busy on:click=act(Intent::Lock)>"Lock identity"</button>
                <p>"Import a copy of an existing encrypted URMA vault. Import validates its password and unlocks its active identity. Your original vault stays intact. An existing imported vault cannot be overwritten."</p>
                <label>"Encrypted URMA vault"<input id=VAULT_INPUT_ID type="file" accept=".urma,.vault" /></label>
                <button class="btn btn-primary"
                    disabled=move || busy() || already_imported(state)
                    on:click=act(Intent::Import)>"Import & unlock vault"</button>
            </SettingsGroup>

            <ProfileEditor />
        </section>
    }
}

fn configured_unlock(state: AppState) -> bool {
    let Some(status) = state.identity.get() else {
        return false;
    };
    status.configured_unlock_available
}

fn already_imported(state: AppState) -> bool {
    let Some(status) = state.identity.get() else {
        return false;
    };
    status.source == "imported"
}

#[component]
fn ActiveIdentity(status: IdentityStatus) -> impl IntoView {
    let session = match status.unlocked {
        true => "Unlocked for this session",
        false => "Locked",
    };
    let origin = match status.source.as_str() {
        "imported" => "Using your imported encrypted vault.",
        "configured" => "Using your configured URMA vault.",
        _ => "Import an encrypted URMA vault to begin.",
    };
    let fingerprint = status.author_fingerprint.clone();
    view! {
        <p>{session}</p>
        <p>{origin}</p>
        {match status.author_pubkey {
            Some(author) => view! {
                <div class="wallet-details-card">
                    <span class="label">"Author public key"</span>
                    <code class="code-val">{author.clone()}</code>
                    <p>{fingerprint.clone()}</p>
                    <a class="btn btn-secondary" href=format!("/profile/{author}")>"View public profile"</a>
                </div>
            }.into_any(),
            None => ().into_any(),
        }}
    }
}
