use crate::components::BackToFeed;
use crate::components::ComposeBox;
use crate::components::Missing;
use crate::components::PostCard;
use crate::components::Warning;
use crate::state::thread::Showing;
use crate::state::thread::Thread;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::Read;
use leptos::prelude::{component, view};

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let thread = Thread::followed();
    view! {
        <section class="page-container post-detail-page">
            <Warning message=thread.trouble />
            <BackToFeed />
            {move || match thread.showing.get() {
                Showing::Loading => view! {
                    <div class="loading-state"><span>"Loading on-chain post record..."</span></div>
                }.into_any(),
                Showing::Post(post) => {
                    let replies = thread.replies;
                    view! {
                        <div class="detail-wrapper">
                            <PostCard post=post.clone() />
                            <div class="reply-compose-section">
                                <ComposeBox />
                            </div>
                            <div class="replies-section">
                                <h3 class="section-title">"Confirmed Replies (" {move || replies.read().len()} ")"</h3>
                                {move || replies.get().into_iter().map(|reply| view! { <PostCard post=reply /> }).collect_view()}
                            </div>
                        </div>
                    }.into_any()
                }
                Showing::Unknown(txid) => view! {
                    <Missing icon="⚠️" heading="Public record not found">
                        <p>"Transaction with TxID " <code>{txid}</code> " is not in the currently indexed confirmed history."</p>
                    </Missing>
                }.into_any(),
            }}
        </section>
    }
}
