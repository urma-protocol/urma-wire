use crate::{platform::tauri_call, state::AppState};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{PublicationQuote, Txid};

#[component]
pub fn ComposeBox(
    #[prop(optional)] target_txid: Option<Txid>,
    #[prop(optional)] on_published: Option<Callback<Txid>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();

    let text = RwSignal::new(String::new());
    let quote = RwSignal::new(Option::<PublicationQuote>::None);
    let is_quoting = RwSignal::new(false);
    let error_msg = RwSignal::new(Option::<String>::None);
    let status_msg = RwSignal::new(Option::<String>::None);

    let max_bytes: usize = if target_txid.is_some() {
        32_728 // 32 KiB cap minus 8B header and 32B target txid
    } else {
        32_760 // 32 KiB cap minus 8B header
    };

    let current_bytes = move || text.read().len();
    let remaining_bytes = move || max_bytes.saturating_sub(current_bytes());
    let is_over_limit = move || current_bytes() > max_bytes;

    let chain = move || state.active_chain.get();
    let target_display = target_txid.map(|t| t.short_display());

    let handle_input = move |ev| {
        text.set(event_target_value(&ev));
        error_msg.set(None);
        status_msg.set(None);
        quote.set(None);
    };

    let handle_quote = move |_| {
        let content = text.get();
        if content.trim().is_empty() {
            error_msg.set(Some(
                "Please write some text before requesting a quote.".into(),
            ));
            return;
        }
        if content.len() > max_bytes {
            error_msg.set(Some(format!(
                "Content exceeds URMA V0 limit of {max_bytes} bytes."
            )));
            return;
        }

        is_quoting.set(true);
        error_msg.set(None);
        let current_chain = chain();
        leptos::task::spawn_local(async move {
            match tauri_call::<PublicationQuote>(
                "quote_publication",
                &json!({
                    "chain": current_chain.code(),
                    "text": content,
                    "fee_rate": 1
                }),
            )
            .await
            {
                Ok(q) => {
                    quote.set(Some(q));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Quote calculation error: {e}")));
                }
            }
            is_quoting.set(false);
        });
    };

    let handle_broadcast = move |_| {
        let content = text.get();
        if content.trim().is_empty() {
            error_msg.set(Some("Cannot publish empty post.".into()));
            return;
        }
        // Safety constraint enforcement:
        status_msg.set(Some(
            "Safety lock active: On-chain broadcast requires explicit owner instruction in accordance with protocol safety policy."
                .into(),
        ));
    };

    let _ = on_published;

    view! {
        <div class="compose-card">
            <div class="compose-header">
                <span class="compose-title">
                    {move || {
                        if target_txid.is_some() {
                            format!("Reply to {}", target_display.as_deref().unwrap_or_default())
                        } else {
                            format!("New Post on {}", chain().network_name())
                        }
                    }}
                </span>
                <span class="chain-badge-small">{move || chain().network_name()}</span>
            </div>

            <textarea
                class="compose-textarea"
                placeholder=if target_txid.is_some() {
                    "Write your reply here... (authenticated directly with author BIP 340 key)"
                } else {
                    "What public-interest record do you want to anchor on-chain?"
                }
                prop:value=move || text.get()
                on:input=handle_input
                rows="4"
            />

            <div class="compose-footer">
                <div class="byte-meter">
                    <span class=move || {
                        if is_over_limit() {
                            "byte-count danger"
                        } else if remaining_bytes() < 1000 {
                            "byte-count warning"
                        } else {
                            "byte-count"
                        }
                    }>
                        {current_bytes} " / " {max_bytes} " bytes"
                    </span>
                    <span class="byte-cap-label">" (32 KiB Taproot Cap)"</span>
                </div>

                <div class="compose-buttons">
                    <button
                        class="btn btn-secondary"
                        disabled=move || text.read().trim().is_empty() || is_over_limit() || is_quoting.get()
                        on:click=handle_quote
                    >
                        {move || if is_quoting.get() { "Calculating..." } else { "Review Fee & Quote" }}
                    </button>
                    <button
                        class="btn btn-primary"
                        disabled=move || text.read().trim().is_empty() || is_over_limit()
                        on:click=handle_broadcast
                    >
                        {if target_txid.is_some() { "Broadcast Reply" } else { "Broadcast Post" }}
                    </button>
                </div>
            </div>

            // Publication Quote Breakdown Card
            {move || {
                quote.get().map(|q| {
                    view! {
                        <div class="quote-card alert alert-info">
                            <h4 class="quote-title">"On-Chain Publication Quote"</h4>
                            <div class="quote-grid">
                                <div class="quote-item">
                                    <span class="label">"Network:"</span>
                                    <strong>{q.network_name.clone()}</strong>
                                </div>
                                <div class="quote-item">
                                    <span class="label">"Payload Size:"</span>
                                    <span>{q.content_bytes} " bytes"</span>
                                </div>
                                <div class="quote-item">
                                    <span class="label">"Fee Rate:"</span>
                                    <span>{q.fee_rate} " sat/vB"</span>
                                </div>
                                <div class="quote-item">
                                    <span class="label">"Estimated Total Fee:"</span>
                                    <strong>{q.estimated_fee_sats} " sats/lits"</strong>
                                </div>
                                <div class="quote-item">
                                    <span class="label">"Wallet Balance:"</span>
                                    <span>{q.spendable_balance_sats} " sats/lits"</span>
                                </div>
                                <div class="quote-item">
                                    <span class="label">"Funds Status:"</span>
                                    {if q.sufficient_balance {
                                        view! { <span class="badge badge-success">"✓ Sufficient Funds"</span> }.into_any()
                                    } else {
                                        view! { <span class="badge badge-danger">"⚠️ Insufficient Balance"</span> }.into_any()
                                    }}
                                </div>
                            </div>
                            {q.warning.map(|w| view! {
                                <p class="quote-warning text-danger">{w}</p>
                            })}
                        </div>
                    }
                })
            }}

            {move || {
                error_msg.get().map(|msg| {
                    view! { <div class="alert alert-error">{msg}</div> }
                })
            }}

            {move || {
                status_msg.get().map(|msg| {
                    view! { <div class="alert alert-warning">{msg}</div> }
                })
            }}
        </div>
    }
}
