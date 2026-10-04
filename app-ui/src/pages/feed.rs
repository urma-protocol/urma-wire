use crate::components::ComposeBox;
use crate::components::ConversationCard;
use crate::components::Empty;
use crate::components::FeedFilter;
use crate::components::PublicationPanel;
use crate::components::Warning;
use crate::config::FEED_PAGE_SIZE;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::Signal;
use leptos::prelude::{component, view};

#[component]
pub fn FeedPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.stop_replying();

    view! {
        <section class="page-container feed-page">
            <ComposeBox />
            <PublicationPanel />
            <div class="posts-stream">
                <div class="stream-toolbar">
                    <h2>"Latest records"</h2>
                    <div class="stream-state">
                        <span class="sync-status" role="status">{move || state.synced.get().summary()}</span>
                        <button type="button" class="btn btn-secondary btn-sm"
                            disabled=move || state.is_loading_feed.get()
                            on:click=move |_clicked| state.refresh_feed()>
                            {move || match state.is_loading_feed.get() { true => "Syncing…", false => "Refresh" }}
                        </button>
                    </div>
                </div>
                <FeedFilter />
                <Warning message=Signal::derive(move || state.synced.get().trouble()) />
                <Stream />
            </div>
        </section>
    }
}

#[component]
fn Stream() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || {
            let threaded = state.conversations();
            if !threaded.is_empty() {
                return view! {
                    <div class="posts-list">
                        {threaded.into_iter().take(FEED_PAGE_SIZE).map(|conversation| view! { <ConversationCard conversation=conversation /> }).collect_view()}
                    </div>
                }.into_any();
            }
            if state.is_loading_feed.get() {
                return view! { <p class="loading-state" role="status">{move || state.synced.get().summary()}</p> }.into_any();
            }
            match state.only_followed.get() {
                true => view! {
                    <Empty heading="Nothing on your topics yet">
                        "No confirmed record carries a wire or hashtag you follow. Show all records or follow more topics."
                    </Empty>
                }.into_any(),
                false => view! {
                    <Empty heading="No confirmed records yet">
                        "Posts appear here once their transactions confirm and the recent history is verified."
                    </Empty>
                }.into_any(),
            }
        }}
    }
}
