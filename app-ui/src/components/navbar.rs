use crate::state::AppState;
use leptos::prelude::*;
use wire_app_shared::Chain;

#[component]
pub fn Navbar() -> impl IntoView {
    let state = expect_context::<AppState>();

    let current_chain = move || state.active_chain.get();

    let on_select_btc = move |_| state.set_chain(Chain::Bitcoin);
    let on_select_ltc = move |_| state.set_chain(Chain::Litecoin);

    let chain_info = move || match state.wallet_summary.get() {
        Some(summary) => {
            if summary.tip_height > 0 {
                format!(
                    "{} · Tip #{} · {} sats/lits",
                    summary.network_name, summary.tip_height, summary.confirmed_balance_sats
                )
            } else {
                format!("{} · Confirmed Only", summary.network_name)
            }
        }
        None => match current_chain() {
            Chain::Bitcoin => "Bitcoin Testnet4 · Confirmed Only".to_string(),
            Chain::Litecoin => "Litecoin Testnet · Confirmed Only".to_string(),
        },
    };

    let profile_href = move || match state.wallet_summary.get() {
        Some(summary) => format!("/profile/{}", summary.author_pubkey),
        None => "/settings".to_string(),
    };

    view! {
        <header class="app-header">
            <div class="header-left">
                <a href="/" class="brand-logo">
                    <span class="logo-badge">"URMA"</span>
                    <span class="logo-text">"Wire"</span>
                </a>
                <div class="chain-selector">
                    <button
                        class=move || {
                            if current_chain() == Chain::Bitcoin {
                                "chain-btn active"
                            } else {
                                "chain-btn"
                            }
                        }
                        on:click=on_select_btc
                    >
                        "Bitcoin Testnet4"
                    </button>
                    <button
                        class=move || {
                            if current_chain() == Chain::Litecoin {
                                "chain-btn active"
                            } else {
                                "chain-btn"
                            }
                        }
                        on:click=on_select_ltc
                    >
                        "Litecoin Testnet"
                    </button>
                </div>
            </div>

            <div class="header-status">
                <span class="status-dot"></span>
                <span class="status-text">{chain_info}</span>
            </div>

            <nav class="header-nav">
                <a href="/" class="nav-link">
                    <span class="nav-icon">"📰"</span>
                    <span class="nav-label">"Feed"</span>
                </a>
                <a href="/compose" class="nav-link">
                    <span class="nav-icon">"✏️"</span>
                    <span class="nav-label">"Publish"</span>
                </a>
                <a href=profile_href class="nav-link">
                    <span class="nav-icon">"👤"</span>
                    <span class="nav-label">"Identity"</span>
                </a>
                <a href="/settings" class="nav-link">
                    <span class="nav-icon">"⚙️"</span>
                    <span class="nav-label">"Wallet"</span>
                </a>
            </nav>
        </header>
    }
}
