use super::feed_topics::FeedTopics;
use crate::{
    components::{ComposeBox, PostCard},
    state::AppState,
};
use leptos::prelude::*;
use wire_app_shared::live_model::{matches_topic, matches_topic_filters};

#[component]
pub fn FeedPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let wire_filter = RwSignal::new(String::new());
    let hashtag_filter = RwSignal::new(String::new());
    let followed = RwSignal::new(Vec::<String>::new());
    let only_followed = RwSignal::new(false);
    let posts = move || {
        state
            .feed
            .get()
            .into_iter()
            .filter(|post| {
                if only_followed.get() {
                    followed
                        .get()
                        .iter()
                        .any(|label| matches_topic(post, label))
                } else {
                    matches_topic_filters(post, &wire_filter.get(), &hashtag_filter.get())
                }
            })
            .take(100)
            .collect::<Vec<_>>()
    };
    let is_loading = move || state.is_loading_feed.get();

    view! {
        <section class="page-container feed-page">
            <div class="feed-header">
                <div>
                    <span class="eyebrow">"PUBLIC WORDS. VERIFIED ORIGIN."</span>
                    <h1 class="page-title">"On the " <span class="accent-text">"wire."</span></h1>
                    <p class="page-subtitle">
                        "A public record, signed by its author. Read what’s been confirmed."
                    </p>
                </div>
            </div>

            <div class="feed-compose-wrapper">
                <ComposeBox />
            </div>

            <div class="posts-stream">
                <FeedTopics chain=state.active_chain wire_filter=wire_filter hashtag_filter=hashtag_filter followed=followed only_followed=only_followed />
                <div class="stream-toolbar"><h2>"Latest records"</h2>
                    <button class="btn btn-secondary btn-sm" disabled=is_loading on:click=move |_| state.load_feed()>
                        {move || if is_loading() { "Syncing…" } else { "Refresh feed" }}
                    </button>
                </div>
                <p class="sync-status" role="status">{move || state.feed_status.get()}</p>
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
                                    <svg class="empty-mark" viewBox="0 0 64 64" aria-hidden="true"><path d="M8 22 32 8l24 14-24 14ZM8 33l24 14 24-14M8 44l24 14 24-14"/></svg>
                                    <h3>"The wire is quiet."</h3>
                                    <p>
                                        "No matching records in the indexed history. Try the global feed or refresh for newly confirmed records."
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
