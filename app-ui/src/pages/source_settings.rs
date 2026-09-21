use crate::platform::{tauri_call, use_platform_bridge, PlatformType};
use leptos::prelude::*;

#[component]
pub fn BrowserSourceSettings() -> impl IntoView {
    let browser = use_platform_bridge().platform_type() == PlatformType::Web;
    let source = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    if browser {
        leptos::task::spawn_local(async move {
            match tauri_call::<String>("get_chain_source", &serde_json::json!({})).await {
                Ok(url) => source.set(url),
                Err(error) => status.set(error),
            }
        });
    }
    let save = move |_| {
        busy.set(true);
        leptos::task::spawn_local(async move {
            match tauri_call::<()>(
                "set_chain_source",
                &serde_json::json!({"url": source.get_untracked()}),
            )
            .await
            {
                Ok(()) => {
                    status.set("Source saved after Litecoin Testnet genesis verification.".into())
                }
                Err(error) => status.set(error),
            }
            busy.set(false);
        });
    };
    view! {
        <Show when=move || browser>
            <div class="settings-group">
                <h3 class="group-title">"Public chain source"</h3>
                <p>"Wire contacts this source directly from your browser. It must allow browser CORS requests. No keys or passwords are sent. Offline history remains available; sync and publication need a reachable source."</p>
                <label>"Litecoin Testnet Esplora API URL"
                    <input type="url" prop:value=move || source.get() on:input=move |event| source.set(event_target_value(&event)) />
                </label>
                <button disabled=move || busy.get() on:click=save>"Verify & save source"</button>
                <p>"Examples: https://litecoinspace.org/testnet/api or https://testnetscan.com/ltc-testnet/api"</p>
                <p role="status">{move || status.get()}</p>
            </div>
        </Show>
    }
}
