use crate::components::ComposeBox;
use crate::components::PostCard;
use crate::limits::FEED_PAGE_SIZE;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::{component, view};

#[component]
pub fn FeedPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.load_feed();
    state.stop_replying();

    view! {
        <section class="page-container feed-page">
            <ComposeBox />
            <div class="posts-stream">
                <div class="stream-toolbar">
                    <h2>"Latest records"</h2>
                </div>
                <SyncTrouble />
                <Stream />
            </div>
        </section>
    }
}

#[component]
fn SyncTrouble() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || {
            let status = state.feed_status.get();
            match status.starts_with("Sync failed") {
                true => view! { <p class="alert alert-error" role="status">{status}</p> }.into_any(),
                false => ().into_any(),
            }
        }}
    }
}

#[component]
fn Stream() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || {
            let posts = state.feed.get();
            if !posts.is_empty() {
                return view! {
                    <div class="posts-list">
                        {posts.into_iter().take(FEED_PAGE_SIZE).map(|post| view! { <PostCard post=post /> }).collect_view()}
                    </div>
                }.into_any();
            }
            match state.is_loading_feed.get() {
                true => view! { <p role="status">"Loading…"</p> }.into_any(),
                false => view! { <p class="empty-state">"No posts yet."</p> }.into_any(),
            }
        }}
    }
}
