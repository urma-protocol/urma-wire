use crate::{platform::tauri_call, state::AppState};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::IdentityStatus;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export async function read_identity_file(limit) {
    const input = document.getElementById('identity-vault-file');
    const file = input.files && input.files[0];
    if (!file) throw 'Choose an encrypted URMA vault first';
    if (file.size > limit) throw 'Encrypted URMA vault exceeds the size limit';
    const bytes = new Uint8Array(await file.arrayBuffer());
    input.value = '';
    return JSON.stringify(Array.from(bytes));
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn read_identity_file(
        limit: usize,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

async fn selected_vault() -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let value = read_identity_file(wire_app_shared::identity::MAX_IDENTITY_VAULT_BYTES)
            .await
            .map_err(|error| {
                error
                    .as_string()
                    .unwrap_or_else(|| "Cannot read the selected vault".into())
            })?;
        serde_json::from_str(&value.as_string().ok_or("Cannot read the selected vault")?)
            .map_err(|_| "Cannot read the selected vault".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Err("File selection requires the app frontend".into())
    }
}

#[component]
pub fn IdentityPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let message = RwSignal::new(Option::<String>::None);
    let run = move |action: &'static str| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        message.set(None);
        let credential = password.get_untracked();
        password.set(String::new());
        leptos::task::spawn_local(async move {
            let result: Result<(), String> = async {
                match action {
                    "import" => {
                        let bytes = selected_vault().await?;
                        let status = tauri_call::<IdentityStatus>(
                            "import_identity",
                            &json!({"vaultBytes": bytes, "password": credential}),
                        )
                        .await?;
                        state.identity.set(Some(status));
                    }
                    "lock" => tauri_call::<()>("lock_wallet", &json!({})).await?,
                    _ => {
                        tauri_call::<()>(
                            "unlock_wallet",
                            &json!({"password": credential, "useUnlockFile": action == "file"}),
                        )
                        .await?
                    }
                }
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {
                    state.wallet_summary.set(None);
                    if action == "lock" {
                        state.identity.update(|status| {
                            if let Some(status) = status {
                                status.unlocked = false;
                                status.author_pubkey = None;
                                status.author_fingerprint = None;
                            }
                        });
                    }
                    state.load_identity();
                    message.set(Some(
                        if action == "lock" {
                            "Identity locked. Already approved signed plans remain resumable."
                        } else {
                            "Identity unlocked for this session."
                        }
                        .into(),
                    ));
                }
                Err(error) => message.set(Some(error)),
            }
            busy.set(false);
        });
    };
    view! {
        <section class="page-container settings-page">
            <div><span class="eyebrow">"YOUR KEYS. YOUR CONTROL."</span><h1 class="page-title">"Your "<span class="accent-text">"identity."</span></h1></div>
            <p class="page-subtitle">"Unlock once to sign during this session. Restarting Wire locks your identity. Funding and network availability are shown in Wallet."</p>
            {move || state.identity_error.get().map(|error| view! { <div class="alert alert-info">{error}</div> })}
            {move || message.get().map(|text| view! { <div class="alert alert-info">{text}</div> })}
            <div class="settings-group">
                <h3 class="group-title">"Active identity"</h3>
                {move || state.identity.get().map(|status| view! {
                    <p>{if status.unlocked { "Unlocked for this session" } else { "Locked" }}</p>
                    <p>{match status.source.as_str() { "imported" => "Using your imported encrypted vault.", "configured" => "Using your configured URMA vault.", _ => "Import an encrypted URMA vault to begin." }}</p>
                    {status.author_pubkey.map(|author| view! {
                        <div class="wallet-details-card"><span class="label">"Author public key"</span><code class="code-val">{author.clone()}</code>
                        <p>{status.author_fingerprint.unwrap_or_default()}</p>
                        <a class="btn btn-secondary" href=format!("/profile/{author}")>"View public profile"</a></div>
                    })}
                })}
                <button class="btn btn-secondary" disabled=move || busy.get() on:click=move |_| state.load_identity()>"Refresh session status"</button>
            </div>
            <div class="settings-group">
                <h3 class="group-title">"Unlock or import"</h3>
                <label>"Vault password"
                    <input type="password" autocomplete="off" prop:value=move || password.get()
                        on:input=move |event| password.set(event_target_value(&event)) />
                </label>
                <button class="btn btn-primary" disabled=move || busy.get() on:click=move |_| run("unlock")>"Unlock identity"</button>
                <Show when=move || state.identity.get().is_some_and(|status| status.configured_unlock_available)>
                    <button class="btn btn-secondary" disabled=move || busy.get() on:click=move |_| run("file")>"Use configured unlock file"</button>
                </Show>
                <button class="btn btn-secondary" disabled=move || busy.get() on:click=move |_| run("lock")>"Lock identity"</button>
                <p>"Import a copy of an existing encrypted URMA vault. Import validates its password and unlocks its active identity. Your original vault stays intact. An existing imported vault cannot be overwritten."</p>
                <label>"Encrypted URMA vault"<input id="identity-vault-file" type="file" accept=".urma,.vault" /></label>
                <button class="btn btn-primary" disabled=move || busy.get() || state.identity.get().is_some_and(|status| status.source == "imported")
                    on:click=move |_| run("import")>"Import & unlock vault"</button>
            </div>
        </section>
    }
}
