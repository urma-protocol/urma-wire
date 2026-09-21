use crate::{platform::tauri_call, state::AppState};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{live_model::Publication, Txid};

#[component]
pub fn ComposeBox(
    #[prop(optional)] target_txid: Option<Txid>,
    #[prop(optional)] on_published: Option<Callback<Txid>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let text = RwSignal::new(String::new());
    let wire = RwSignal::new(String::new());
    let hashtags = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let max_bytes = if target_txid.is_some() {
        32_728
    } else {
        32_760
    };
    let prepare = move |_| {
        busy.set(true);
        error.set(None);
        let content = text.get();
        let wire = wire
            .get()
            .trim()
            .trim_start_matches('/')
            .to_ascii_lowercase();
        let mut hashtags: Vec<String> = hashtags
            .get()
            .split_whitespace()
            .map(|tag| tag.trim_start_matches('#').to_ascii_lowercase())
            .collect();
        hashtags.sort();
        hashtags.dedup();
        let chain = state.active_chain.get();
        leptos::task::spawn_local(async move {
            match tauri_call::<Publication>(
                "quote_publication",
                &json!({
                    "chain": chain.code(), "text": content,
                    "wire": wire, "hashtags": hashtags,
                    "target": target_txid.map(|t| t.to_display_hex())
                }),
            )
            .await
            {
                Ok(quote) => state.publication.set(Some(quote)),
                Err(message) => error.set(Some(message)),
            }
            busy.set(false);
        });
    };
    let _ = on_published;
    view! {
        <div class="compose-card">
            <div class="compose-header"><strong>{if target_txid.is_some() { "Write a reply" } else { "Put it on record" }}</strong><span class="chain-badge-small">"PUBLIC"</span></div>
            <textarea class="compose-textarea" rows="4" aria-label="Post text" placeholder="What’s worth putting on the wire?"
                prop:value=move || text.get() on:input=move |ev| text.set(event_target_value(&ev)) />
            <div class="topic-fields">
                <label>"Wire (optional)"<input class="input" placeholder="/rosint" prop:value=move || wire.get() on:input=move |ev| wire.set(event_target_value(&ev)) /></label>
                <label>"Hashtags (optional)"<input class="input" placeholder="#investigation #documents" prop:value=move || hashtags.get() on:input=move |ev| hashtags.set(event_target_value(&ev)) /></label>
            </div>
            <p class="action-desc">"Open labels, no registration or affiliation. Up to 8 hashtags; 1–32 lowercase letters, digits, - or _. Labels are lowercased and duplicate hashtags removed before review."</p>
            <div class="compose-footer">
                <span class="byte-meter">{move || text.read().len()} " text bytes · 32 KiB total including labels"</span>
                <button class="btn btn-primary" on:click=prepare
                    disabled=move || { busy.get() || state.following.get() || text.read().trim().is_empty() || text.read().len() > max_bytes }>
                    {move || if busy.get() { "Preparing exact quote…" } else { "Review & publish" }}
                </button>
            </div>
            {move || error.get().map(|message| view! { <div class="alert alert-error">{message}</div> })}
        </div>
    }
}

#[component]
pub fn PublicationPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || state.publication.get().map(|publication| {
            let id = publication.id.clone();
            let approve = !publication.approved;
            view! {
                <section class="compose-card publication-card">
                    <div class="compose-header"><strong>{if publication.complete { "On the record" } else { "Your publication" }}</strong><span class="chain-badge-small">"LTC TESTNET"</span></div>
                    <p class="publication-text">{publication.text}</p>
                    <p class="topic-labels">{if publication.wire.is_empty() { "No wire".into() } else { format!("/{}", publication.wire) }} " " {publication.hashtags.iter().map(|tag| format!("#{tag}")).collect::<Vec<_>>().join(" ")}</p>
                    {publication.target.map(|target| view! { <p>"Reply to: " <code>{target}</code></p> })}
                    <div class="publication-costs">
                        <div><span>"Network fee"</span><strong class="accent-text">{publication.fee}<small>" litoshi"</small></strong></div>
                        <div><span>"Selected funding"</span><strong>{publication.selected_funding}<small>" litoshi"</small></strong></div>
                        <div><span>"Returned to you"</span><strong>{publication.returned}<small>" litoshi"</small></strong></div>
                    </div>
                    <div class="publication-status" role="status"><span>{publication.confirmed} "/2 confirmed"</span><span>{publication.mempool} " in mempool"</span></div>
                    <progress class="publication-progress" max="2" value=publication.confirmed aria-label="Confirmed transactions"></progress>
                    <p class="action-desc">{publication.message}</p>
                    <details class="transaction-details"><summary>"Transaction details"</summary><code>{publication.txid}</code></details>
                    {(!publication.complete).then(|| view! {
                        <p class="action-desc">"Approval publishes this exact text, wire and hashtags and spends the fee above. Changes in the composer require a new review."</p>
                        <div class="publication-actions">
                        <button class="btn btn-primary" disabled=move || state.following.get()
                            on:click=move |_| state.follow(id.clone(), approve)>
                            {if approve { "Approve fee & publish" } else { "Resume publication" }}
                        </button>
                        <button class="btn btn-secondary" disabled=move || !state.following.get()
                            on:click=move |_| state.pause_requested.set(true)>"Pause after current request"</button>
                        </div>
                        <p class="action-desc">"You can close Wire and resume later with the same plan and fee."</p>
                    })}
                </section>
            }
        })}
        {move || state.publication_error.get().map(|error| view! { <div class="alert alert-error">{error}</div> })}
    }
}
