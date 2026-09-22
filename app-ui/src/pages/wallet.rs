use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn WalletPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let wallet_summary = move || state.wallet_summary.get();
    let is_loading_wallet = move || state.is_loading_wallet.get();
    let status_msg = move || state.status_message.get();

    let copy_feedback = RwSignal::new(Option::<String>::None);

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
                                        <span class="stat-value">{format!(
                                            "{}.{:08} {}",
                                            summary.confirmed_balance_sats / 100_000_000,
                                            summary.confirmed_balance_sats % 100_000_000,
                                            summary.chain.code().to_ascii_uppercase(),
                                        )}</span>
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
        </section>
    }
}
