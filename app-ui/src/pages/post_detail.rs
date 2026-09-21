use crate::{
    components::{ComposeBox, PostCard},
    platform::tauri_call,
    state::AppState,
};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use wire_app_shared::{Post, Txid};

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();

    let txid_str = move || params.read().get("txid").unwrap_or_default();

    let parsed_txid = move || {
        let s = txid_str();
        Txid::from_display_hex(&s).ok()
    };

    let post = RwSignal::new(Option::<Post>::None);
    let is_loading = RwSignal::new(true);
    let replies = RwSignal::new(Vec::<Post>::new());
    let load_error = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        let chain = state.active_chain.get();
        if let Some(txid) = parsed_txid() {
            is_loading.set(true);
            load_error.set(None);
            replies.set(Vec::new());
            leptos::task::spawn_local(async move {
                match tauri_call::<Option<Post>>(
                    "get_post",
                    &serde_json::json!({
                        "chain": chain.code(),
                        "txid": txid.to_display_hex()
                    }),
                )
                .await
                {
                    Ok(p) => post.set(p),
                    Err(error) => {
                        post.set(None);
                        load_error.set(Some(error));
                    }
                }
                match tauri_call::<Vec<Post>>(
                    "get_replies",
                    &serde_json::json!({
                        "chain": chain.code(), "txid": txid.to_display_hex()
                    }),
                )
                .await
                {
                    Ok(items) => replies.set(items),
                    Err(error) => load_error.set(Some(error)),
                }
                is_loading.set(false);
            });
        } else {
            is_loading.set(false);
            post.set(None);
        }
    });

    view! {
        <section class="page-container post-detail-page">
            {move || load_error.get().map(|error| view! { <div class="alert alert-error">{error}</div> })}
            <div class="page-nav-back">
                <a href="/" class="back-link">"← Back to feed"</a>
            </div>

            {move || {
                if is_loading.get() {
                    view! {
                        <div class="loading-state">
                            <span>"Loading on-chain post record..."</span>
                        </div>
                    }.into_any()
                } else {
                    match post.get() {
                        Some(p) => {
                            let target_txid = p.txid;
                            view! {
                                <div class="detail-wrapper">
                                    <PostCard post=p />

                                    <div class="reply-compose-section">
                                        <ComposeBox target_txid=target_txid />
                                    </div>

                                    <div class="replies-section">
                                        <h3 class="section-title">
                                            "Confirmed Replies (" {move || replies.read().len()} ")"
                                        </h3>
                                        {move || replies.get().into_iter().map(|reply| view! { <PostCard post=reply /> }).collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        }
                        None => {
                            let raw = txid_str();
                            let chain = state.active_chain.get().network_name();
                            view! {
                                <div class="empty-state not-found">
                                    <span class="empty-icon">"⚠️"</span>
                                    <h3>"Public record not found"</h3>
                                    <p>
                                        "Transaction with TxID " <code>{raw}</code>
                                        " is not in the currently indexed confirmed history of "
                                        <strong>{chain}</strong> "."
                                    </p>
                                    <a href="/" class="btn btn-secondary">"Return to feed"</a>
                                </div>
                            }.into_any()
                        }
                    }
                }
            }}
        </section>
    }
}
