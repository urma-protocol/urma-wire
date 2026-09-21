use crate::{
    components::{ComposeBox, PostCard},
    state::AppState,
};
use leptos::prelude::*;

#[component]
pub fn FeedPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.load_feed();

    view! {
        <section class="page-container feed-page">
            <ComposeBox />
            <div class="posts-stream">
                <div class="stream-toolbar">
                    <h2>"Latest records"</h2>
                </div>
                {move || {
                    let status = state.feed_status.get();
                    status.starts_with("Sync failed").then(|| view! {
                        <p class="alert alert-error" role="status">{status}</p>
                    })
                }}
                {move || {
                    let posts = state.feed.get();
                    if !posts.is_empty() {
                        view! {
                            <div class="posts-list">
                                {posts.into_iter().take(100).map(|post| view! {
                                    <PostCard post=post />
                                }).collect_view()}
                            </div>
                        }.into_any()
                    } else if state.is_loading_feed.get() {
                        view! { <p role="status">"Loading…"</p> }.into_any()
                    } else {
                        view! { <p class="empty-state">"No posts yet."</p> }.into_any()
                    }
                }}
            </div>
        </section>
    }
}
