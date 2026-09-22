use crate::{platform::use_platform_bridge, state::AppState};
use leptos::prelude::*;
use wire_app_shared::Chain;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let platform = use_platform_bridge();

    let current_chain = move || state.active_chain.get();
    let hide_images = move || !state.autoload_images.get();
    let status_msg = move || state.status_message.get();

    let on_toggle_autoload = move |_| {
        let enabled = !state.autoload_images.get_untracked();
        match crate::components::markdown::save_autoload(enabled) {
            Ok(()) => state.autoload_images.set(enabled),
            Err(error) => state.status_message.set(Some(error)),
        }
    };

    view! {
        <section class="page-container settings-page">
            <div><span class="eyebrow">"PREFERENCES."</span><h1 class="page-title">"Your " <span class="accent-text">"settings."</span></h1></div>
            <p class="page-subtitle">
                "Network, media privacy and runtime preferences. Funding and balances live in Wallet."
            </p>

            {move || {
                status_msg().map(|msg| {
                    view! {
                        <div class="alert alert-info">
                            <span>{msg}</span>
                        </div>
                    }
                })
            }}

            // Active Network Selection
            <div class="settings-group">
                <h3 class="group-title">"Network"</h3>
                <p class="group-desc">
                    "This V0 flow uses Litecoin Testnet. The selected node's genesis is checked before operations."
                </p>

                <div class="radio-options">
                    <label class="radio-label">
                        <input
                            type="radio"
                            name="chain"
                            checked=move || current_chain() == Chain::Litecoin
                            on:change=move |_| state.set_chain(Chain::Litecoin)
                        />
                        <div class="radio-content">
                            <strong>"Litecoin Testnet"</strong>
                            <span>"Uses the URMA node configuration; public sources when no local RPC is configured."</span>
                        </div>
                    </label>

                    <label class="radio-label">
                        <input
                            type="radio"
                            name="chain"
                            checked=move || current_chain() == Chain::Bitcoin
                            disabled=true
                            on:change=move |_| state.set_chain(Chain::Bitcoin)
                        />
                        <div class="radio-content">
                            <strong>"Bitcoin Testnet4"</strong>
                            <span>"Not connected in this application milestone."</span>
                        </div>
                    </label>
                </div>
            </div>

            // External Media Privacy (WIRE.md D-020)
            <div class="settings-group">
                <h3 class="group-title">"External Media Fetch (Markdown Images)"</h3>
                <p class="group-desc">
                    "Under the URMA Wire specification (D-020), external media is not stored on-chain."
                </p>

                <label class="toggle-control">
                    <input
                        type="checkbox"
                        checked=hide_images
                        on:change=on_toggle_autoload
                    />
                    <span class="toggle-text">"Don't autoload external images"</span>
                </label>

                <div class="privacy-notice">
                    <strong>"External media privacy: "</strong>
                    "Downloading external images exposes your connection IP address and request metadata to the host server. Disabling autoload prevents automatic metadata leakage."
                </div>
                <p class="action-desc">"Desktop fetches checked raster images. Browser fetching is disabled because destination checks cannot be enforced. Displayed media is temporary; Wire keeps no media cache."</p>
                <button class="btn btn-secondary" on:click=move |_| {
                    state.media_epoch.update(|epoch| *epoch += 1);
                    state.media_bytes.set(0);
                }>"Clear displayed media"</button>
            </div>

            // Runtime Platform & Indexer
            <super::source_settings::BrowserSourceSettings />
            <div class="settings-group">
                <h3 class="group-title">"Runtime Platform & Local Indexer"</h3>
                <div class="runtime-info-table">
                    <div class="info-row">
                        <span class="info-label">"Runtime Target:"</span>
                        <span class="info-val">{platform.platform_name()}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"Indexing Engine:"</span>
                        <span class="info-val">{platform.storage_name()}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">"URMA Protocol:"</span>
                        <span class="info-val">"URMA V0"</span>
                    </div>
                </div>
            </div>
        </section>
    }
}
