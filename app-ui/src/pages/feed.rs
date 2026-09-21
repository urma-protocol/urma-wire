use crate::{
    components::{ComposeBox, PostCard},
    state::AppState,
};
use leptos::prelude::*;

#[component]
pub fn FeedPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let posts = move || state.feed.get();
    let is_loading = move || state.is_loading_feed.get();
    let chain_name = move || state.active_chain.get().network_name();

    view! {
        <section class="page-container feed-page">
            <div class="feed-header">
                <div>
                    <h1 class="page-title">"Public Feed — " {chain_name}</h1>
                    <p class="page-subtitle">
                        "Press records and public interest reports authenticated directly with author BIP 340 Taproot keys."
                    </p>
                </div>
            </div>

            <div class="feed-compose-wrapper">
                <ComposeBox />
            </div>

            <div class="posts-stream">
                {move || {
                    if is_loading() {
                        view! {
                            <div class="loading-state">
                                <span>"Refreshing confirmed feed from local indexer..."</span>
                            </div>
                        }.into_any()
                    } else {
                        let list = posts();
                        if list.is_empty() {
                            view! {
                                <div class="empty-state">
                                    <span class="empty-icon">"📭"</span>
                                    <h3>"No confirmed records on this chain"</h3>
                                    <p>
                                        "Empty confirmed feed. Posts become visible only after their first on-chain confirmation (D-017)."
                                    </p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="posts-list">
                                    {list.into_iter().map(|p| {
                                        view! { <PostCard post=p /> }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </div>
        </section>
    }
}
