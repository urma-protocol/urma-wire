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

    Effect::new(move |_| {
        let chain = state.active_chain.get();
        if let Some(txid) = parsed_txid() {
            is_loading.set(true);
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
                    Err(_) => post.set(None),
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
                                            "Confirmed Replies (0)"
                                        </h3>
                                        <p class="empty-replies">
                                            "No confirmed replies for this post yet."
                                        </p>
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
                                        " does not exist or is not confirmed on "
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
