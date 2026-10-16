use crate::components::Actions;
use crate::components::Fact;
use crate::components::Facts;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::components::Stat;
use crate::config::COPY_FEEDBACK_MS;
use crate::platform::copy_to_clipboard;
use crate::state::AppState;
use crate::state::Said;
use leptos::prelude::expect_context;
use leptos::prelude::Callable;
use leptos::prelude::Callback;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, view};
use wire_app_core::model::WalletSummary;

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
        copied.set(Said::Something("Address copied".to_owned()));
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(COPY_FEEDBACK_MS).await;
            copied.set(Said::Nothing);
        });
    });

    view! {
        <section class="page-container settings-page">
            <PageHeader eyebrow="PUBLIC FUNDING." title="Your" accent="wallet.">
                "Your selected URMA identity signs posts and pays their fees. Its funding address and balance are public."
            </PageHeader>
            <Notice message=state.status_message.into() />

            <SettingsGroup title="Funding wallet">
                <Actions>
                    <button class="btn btn-secondary btn-sm"
                        disabled=move || state.is_loading_wallet.get()
                        on:click=refresh>
                        {move || match state.is_loading_wallet.get() {
                            true => "Refreshing…",
                            false => "Refresh balance",
                        }}
                    </button>
                    <a class="btn btn-secondary btn-sm" href="/identity">"Manage identity"</a>
                </Actions>
                {move || match state.wallet_summary.get() {
                    Some(summary) => view! { <Funding summary=summary copied=copied on_copy=copy_address /> }.into_any(),
                    None => view! { <Unfunded /> }.into_any(),
                }}
                <p class="action-desc">
                    "The active identity in the existing vault signs and funds publication. Only confirmed spendable outputs count, and a sufficient total balance does not guarantee a single usable funding output. Private Archive recovery secrets are separate."
                </p>
            </SettingsGroup>
        </section>
    }
}

#[component]
fn Unfunded() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <p class="group-desc" role="status">{move || unfunded(state)}</p>
    }
}

fn unfunded(state: AppState) -> &'static str {
    let Some(status) = state.identity.get() else {
        return "Unlock your identity to query funding.";
    };
    match status.unlocked() {
        true => "Balance unavailable. Your identity remains unlocked; refresh to query funding.",
        false => "Unlock your identity to query funding.",
    }
}

#[component]
fn Funding(summary: WalletSummary, copied: RwSignal<Said>, on_copy: Callback<()>) -> impl IntoView {
    let balance = summary.chain.amount(summary.confirmed_balance_sats);
    view! {
        <Facts>
            <Fact label="Fingerprint">
                <span class="badge badge-neutral">{summary.author_fingerprint.clone()}</span>
            </Fact>
            <Fact label="Chain tip">{format!("#{}", summary.tip_height)}</Fact>
            <Fact label="Author public key (BIP 340)" wide=true>
                <code class="code-val">{summary.author_pubkey.clone()}</code>
            </Fact>
            <Fact label="Receive address" wide=true>
                <div class="address-copy-row">
                    <code class="code-val">{summary.receive_address.clone()}</code>
                    <button class="btn btn-secondary btn-xs" on:click=move |_clicked| on_copy.run(())>"Copy"</button>
                </div>
                {move || match copied.get() {
                    Said::Something(message) => view! { <span class="copy-feedback-text" role="status">{message}</span> }.into_any(),
                    Said::Nothing => ().into_any(),
                }}
            </Fact>
        </Facts>
        <div class="wallet-grid">
            <Stat label="Confirmed balance" value=balance />
            <Stat label="Spendable outputs" value=summary.utxo_count.to_string() />
        </div>
    }
}
