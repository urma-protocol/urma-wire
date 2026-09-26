use crate::components::CompactReply;
use crate::components::PostCard;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::{component, view};
use wire_app_core::feed::Conversation;
use wire_app_core::model::{Post, Txid};

#[component]
pub fn ConversationCard(conversation: Conversation) -> impl IntoView {
    let Conversation {
        post,
        replies,
        unseen,
    } = conversation;
    let txid = post.txid;
    view! {
        <div class="conversation">
            <PostCard post=post />
            <Replies txid=txid replies=replies unseen=unseen />
        </div>
    }
}

#[component]
fn Replies(txid: Txid, replies: Vec<Post>, unseen: usize) -> impl IntoView {
    if replies.is_empty() {
        return ().into_any();
    }
    let total = replies.len() + unseen;
    view! {
        <div class="conversation-replies">
            {replies.into_iter().map(|reply| view! { <CompactReply post=reply /> }).collect_view()}
            <WholeThread txid=txid unseen=unseen total=total />
        </div>
    }
    .into_any()
}

#[component]
fn WholeThread(txid: Txid, unseen: usize, total: usize) -> impl IntoView {
    if unseen == 0 {
        return ().into_any();
    }
    view! {
        <a href=format!("/post/{}", txid.to_display_hex()) class="conversation-more">
            "View all "{total}" replies →"
        </a>
    }
    .into_any()
}
