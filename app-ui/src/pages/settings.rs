use crate::{platform::use_platform_bridge, state::AppState};
use leptos::prelude::*;
use wire_app_shared::Chain;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let platform = use_platform_bridge();

    let current_chain = move || state.active_chain.get();
    let autoload = move || state.autoload_images.get();
    let wallet_summary = move || state.wallet_summary.get();
    let is_loading_wallet = move || state.is_loading_wallet.get();
    let status_msg = move || state.status_message.get();

    let copy_feedback = RwSignal::new(Option::<String>::None);

    let on_toggle_autoload = move |_| {
        state.autoload_images.update(|val| *val = !*val);
    };

    let on_refresh_wallet = move |_| {
        state.load_wallet();
    };

    let on_copy_address = move |_| {
        if let Some(summary) = wallet_summary() {
            crate::platform::copy_to_clipboard(&summary.receive_address);
            copy_feedback.set(Some("Address copied to clipboard!".into()));
        }
    };

    view! {
        <section class="page-container settings-page">
            <div><span class="eyebrow">"PUBLIC FUNDING."</span><h1 class="page-title">"Your " <span class="accent-text">"wallet."</span></h1></div>
            <p class="page-subtitle">
                "Your selected URMA identity signs posts and pays their fees. Its funding address and balance are public."
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

            // Funding Wallet & Key Derivation
            <div class="settings-group">
                <a class="btn btn-secondary" href="/identity">"Manage identity"</a>
                <div class="group-header-row">
                    <h3 class="group-title">"Funding wallet"</h3>
                    <button
                        class="btn btn-secondary btn-sm"
                        disabled=is_loading_wallet
                        on:click=on_refresh_wallet
                    >
                        {move || if is_loading_wallet() { "Refreshing..." } else { "Refresh Balance" }}
                    </button>
                </div>

                {move || {
                    match wallet_summary() {
                        Some(summary) => view! {
                            <div class="wallet-details-card">
                                <div class="wallet-row">
                                    <span class="label">"Author Public Key (BIP 340 Taproot):"</span>
                                    <code class="code-val">{summary.author_pubkey.clone()}</code>
                                </div>
                                <div class="wallet-row">
                                    <span class="label">"Author Fingerprint:"</span>
                                    <span class="badge badge-neutral">"🔑 " {summary.author_fingerprint.clone()}</span>
                                </div>
                                <div class="wallet-row">
                                    <span class="label">"Receive Address (" {summary.network_name.clone()} "):"</span>
                                    <div class="address-copy-row">
                                        <code class="code-val address">{summary.receive_address.clone()}</code>
                                        <button class="btn btn-secondary btn-xs" on:click=on_copy_address>
                                            "Copy"
                                        </button>
                                    </div>
                                    {move || copy_feedback.get().map(|f| view! {
                                        <span class="copy-feedback-text">{f}</span>
                                    })}
                                </div>
                                <div class="wallet-grid">
                                    <div class="grid-stat">
                                        <span class="stat-label">"Confirmed Balance"</span>
                                        <span class="stat-value">{summary.confirmed_balance_sats} " litoshi"</span>
                                    </div>
                                    <div class="grid-stat">
                                        <span class="stat-label">"Funding policy"</span>
                                        <span class="stat-value">"Confirmed spendable outputs only"</span>
                                    </div>
                                    <div class="grid-stat">
                                        <span class="stat-label">"Spendable UTXOs"</span>
                                        <span class="stat-value">{summary.utxo_count}</span>
                                    </div>
                                    <div class="grid-stat">
                                        <span class="stat-label">"Chain Tip Height"</span>
                                        <span class="stat-value">"#" {summary.tip_height}</span>
                                    </div>
                                </div>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="loading-state">
                                <span>{move || if state.identity.get().is_some_and(|status| status.unlocked) {
                                    "Balance unavailable. Your identity remains unlocked; refresh to query funding."
                                } else { "Unlock your identity to query funding." }}</span>
                            </div>
                        }.into_any(),
                    }
                }}

                <p class="action-desc">
                    "The active identity in the existing vault signs and funds publication. A sufficient total balance does not guarantee a single usable funding output. Private Archive recovery secrets are separate."
                </p>
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
                        checked=autoload
                        on:change=on_toggle_autoload
                    />
                    <span class="toggle-text">"Autoload external Markdown images"</span>
                </label>

                <div class="privacy-notice">
                    <strong>"External media privacy: "</strong>
                    "Downloading external images exposes your connection IP address and request metadata to the host server. Disabling autoload prevents automatic metadata leakage."
                </div>
            </div>

            // Runtime Platform & Indexer
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
