use crate::components::DetailRow;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::Said;
use crate::components::SettingsGroup;
use crate::components::Stat;
use crate::platform::copy_to_clipboard;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::Callable;
use leptos::prelude::Callback;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, view};
use wire_app_shared::WalletSummary;

#[component]
pub fn WalletPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let copied = RwSignal::new(Said::Nothing);

    let refresh = move |_clicked: leptos::ev::MouseEvent| {
        state.load_wallet();
    };
    let copy_address = Callback::new(move |_asked: ()| {
        let Some(summary) = state.wallet_summary.get_untracked() else {
            return;
        };
        copy_to_clipboard(&summary.receive_address);
        copied.set(Said::Something("Address copied to clipboard!".to_owned()));
    });

    view! {
        <section class="page-container settings-page">
            <PageHeader
                eyebrow="PUBLIC FUNDING."
                title="Your"
                accent="wallet."
                subtitle="Your selected URMA identity signs posts and pays their fees. Its funding address and balance are public."
            />
            <Notice message=state.status_message.into() />

            <SettingsGroup title="Funding wallet">
                <a class="btn btn-secondary" href="/identity">"Manage identity"</a>
                <div class="group-header-row">
                    <button class="btn btn-secondary btn-sm"
                        disabled=move || state.is_loading_wallet.get()
                        on:click=refresh>
                        {move || match state.is_loading_wallet.get() {
                            true => "Refreshing...",
                            false => "Refresh Balance",
                        }}
                    </button>
                </div>
                {move || match state.wallet_summary.get() {
                    Some(summary) => view! { <Funding summary=summary copied=copied on_copy=copy_address /> }.into_any(),
                    None => view! { <Unfunded /> }.into_any(),
                }}
                <p class="action-desc">
                    "The active identity in the existing vault signs and funds publication. A sufficient total balance does not guarantee a single usable funding output. Private Archive recovery secrets are separate."
                </p>
            </SettingsGroup>
        </section>
    }
}

#[component]
fn Unfunded() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <div class="loading-state">
            <span>{move || match state.identity.get() {
                Some(status) if status.unlocked => "Balance unavailable. Your identity remains unlocked; refresh to query funding.",
                Some(_) => "Unlock your identity to query funding.",
                None => "Unlock your identity to query funding.",
            }}</span>
        </div>
    }
}

#[component]
fn Funding(summary: WalletSummary, copied: RwSignal<Said>, on_copy: Callback<()>) -> impl IntoView {
    let balance = whole_coins(summary.confirmed_balance_sats, summary.chain.code());
    view! {
        <div class="wallet-details-card">
            <DetailRow label="Author Public Key (BIP 340 Taproot):">
                <code class="code-val">{summary.author_pubkey.clone()}</code>
            </DetailRow>
            <DetailRow label="Author Fingerprint:">
                <span class="badge badge-neutral">"🔑 " {summary.author_fingerprint.clone()}</span>
            </DetailRow>
            <DetailRow label="Receive Address:">
                <div class="address-copy-row">
                    <code class="code-val address">{summary.receive_address.clone()}</code>
                    <button class="btn btn-secondary btn-xs" on:click=move |_| on_copy.run(())>"Copy"</button>
                </div>
                {move || match copied.get() {
                    Said::Something(message) => view! { <span class="copy-feedback-text">{message}</span> }.into_any(),
                    Said::Nothing => ().into_any(),
                }}
            </DetailRow>
            <div class="wallet-grid">
                <Stat label="Confirmed Balance" value=balance />
                <Stat label="Funding policy" value="Confirmed spendable outputs only".to_owned() />
                <Stat label="Spendable UTXOs" value=summary.utxo_count.to_string() />
                <Stat label="Chain Tip Height" value=format!("#{}", summary.tip_height) />
            </div>
        </div>
    }
}

fn whole_coins(sats: u64, code: &str) -> String {
    let coins = sats / 100_000_000;
    let fraction = sats % 100_000_000;
    format!("{coins}.{fraction:08} {}", code.to_ascii_uppercase())
}
