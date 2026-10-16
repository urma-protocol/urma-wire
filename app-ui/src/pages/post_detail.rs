use crate::components::BackToFeed;
use crate::components::Missing;
use crate::components::PostCard;
use crate::components::PublicationPanel;
use crate::components::Warning;
use crate::state::thread::Showing;
use crate::state::thread::Thread;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::Read;
use leptos::prelude::{component, view};
use wire_app_core::publication::Kind;

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let thread = Thread::followed();
    view! {
        <section class="page-container post-detail-page">
            <BackToFeed />
            <Warning message=thread.trouble />
            <PublicationPanel kind=Kind::Post />
            {move || match thread.showing.get() {
                Showing::Loading => view! {
                    <p class="loading-state" role="status">"Reading the confirmed record…"</p>
                }.into_any(),
                Showing::Post(post) => {
                    let replies = thread.replies;
                    view! {
                        <div class="detail-wrapper">
                            <PostCard post=post.clone() />
                            <div class="replies-section">
                                <h3 class="section-title">"Replies"<small>{move || replies.read().len()}</small></h3>
                                {move || {
                                    let found = replies.get();
                                    match found.is_empty() {
                                        true => view! { <p class="group-desc">"No confirmed replies yet."</p> }.into_any(),
                                        false => found.into_iter().map(|reply| view! { <PostCard post=reply /> }).collect_view().into_any(),
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any()
                }
                Showing::Unknown(txid) => view! {
                    <Missing heading="Public record not found">
                        <p>"Transaction with TxID " <code>{txid}</code> " is not in the currently indexed confirmed history."</p>
                    </Missing>
                }.into_any(),
            }}
        </section>
    }
}
