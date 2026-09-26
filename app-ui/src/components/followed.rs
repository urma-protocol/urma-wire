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
use leptos::prelude::{component, view};

#[derive(Clone, Copy)]
enum Look {
    Chip,
    Inline,
}

impl Look {
    fn class(self, followed: bool) -> &'static str {
        match (self, followed) {
            (Look::Chip, true) => "topic-chip followed",
            (Look::Chip, false) => "topic-chip",
            (Look::Inline, true) => "topic-inline followed",
            (Look::Inline, false) => "topic-inline",
        }
    }
}

#[component]
pub fn FollowChip(spelled: String) -> impl IntoView {
    let written = spelled.clone();
    view! { <Following spelled=spelled written=written look=Look::Chip /> }
}

#[component]
pub fn FollowToken(spelled: String, written: String) -> impl IntoView {
    view! { <Following spelled=spelled written=written look=Look::Inline /> }
}

#[component]
fn Following(spelled: String, written: String, look: Look) -> impl IntoView {
    let state = expect_context::<AppState>();
    let held = spelled.clone();
    view! {
        <button type="button"
            class=move || look.class(state.topics.get().follows(&held))
            title="Follow or unfollow this topic"
            on:click=move |_clicked| state.toggle_follow(spelled.clone())>
            {written}
        </button>
    }
}

#[component]
pub fn FollowedTopics() -> impl IntoView {
    let state = expect_context::<AppState>();
    let followed = move || state.topics.get().followed_labels();
    view! {
        {move || {
            let held = followed();
            if held.is_empty() {
                return view! {
                    <p class="followed-empty">
                        "Follow a wire or hashtag from any post to collect it here."
                    </p>
                }.into_any();
            }
            view! {
                <div class="followed-strip">
                    {held.into_iter().map(|spelled| view! { <FollowChip spelled=spelled /> }).collect_view()}
                </div>
            }.into_any()
        }}
    }
}

#[component]
pub fn FeedFilter() -> impl IntoView {
    let state = expect_context::<AppState>();
    let followed = move || state.topics.get().followed_labels();
    view! {
        {move || {
            let held = followed();
            if held.is_empty() {
                return ().into_any();
            }
            view! {
                <div class="feed-filter">
                    <button type="button"
                        class=move || match state.only_followed.get() {
                            true => "feed-filter-toggle active",
                            false => "feed-filter-toggle",
                        }
                        on:click=move |_clicked| state.toggle_only_followed()>
                        {move || match state.only_followed.get() {
                            true => "Following",
                            false => "All records",
                        }}
                    </button>
                    <div class="feed-filter-chips">
                        {held.into_iter().map(|spelled| view! { <FollowChip spelled=spelled /> }).collect_view()}
                    </div>
                </div>
            }.into_any()
        }}
    }
}
