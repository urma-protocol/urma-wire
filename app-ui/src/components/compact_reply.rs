use crate::components::markdown::PostText;
use crate::components::post_parts::{InlineReply, ReplyToggle, ThreadCount};
use crate::components::AvatarView;
use crate::config::REPLY_AVATAR_PIXELS;
use crate::state::session::Named;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::{component, view};
use wire_app_core::model::{AuthorId, Inclusion, Post, Txid};

#[component]
pub fn CompactReply(post: Post) -> impl IntoView {
    let author = post.author;
    let author_hex = author.to_hex();
    let profile = format!("/profile/{author_hex}");
    let txid = post.txid;
    let txid_disp = txid.to_display_hex();
    let count = post.reply_count;
    let stamp = match &post.inclusion {
        Inclusion::Confirmed(status) => {
            format!("#{} · {} conf", status.block_height, status.confirmations)
        }
        Inclusion::Mempool => "pending · mempool".to_owned(),
    };
    view! {
        <article class="reply-compact">
            <a href=profile.clone() class="reply-compact-avatar" tabindex="-1" aria-hidden="true">
                <AvatarView author=author size=REPLY_AVATAR_PIXELS />
            </a>
            <div class="reply-compact-head">
                <a href=profile class="reply-compact-author" title=format!("BIP 340 x-only public key: {author_hex}")>
                    <Signer author=author />
                </a>
                <a href=format!("/post/{txid_disp}") class="reply-compact-stamp" title=format!("Reveal TxID: {txid_disp}")>
                    {stamp}
                </a>
            </div>
            <div class="reply-compact-body">
                <PostText text=post.text.clone() wire=post.wire.clone() hashtags=post.hashtags.clone() />
            </div>
            <div class="reply-compact-actions">
                <ReplyToggle post=post.clone() />
                <Answered txid=txid count=count />
            </div>
            <InlineReply post=post />
        </article>
    }
}

#[component]
fn Signer(author: AuthorId) -> impl IntoView {
    let state = expect_context::<AppState>();
    let fingerprint = author.fingerprint();
    view! {
        {move || match state.named(author) {
            Named::Nickname(nickname) => view! {
                <span class="reply-compact-name">{nickname}</span>
                <span class="reply-compact-fingerprint">{fingerprint.clone()}</span>
            }.into_any(),
            Named::Unnamed => view! {
                <span class="reply-compact-name">{fingerprint.clone()}</span>
            }.into_any(),
        }}
    }
}

#[component]
fn Answered(txid: Txid, count: usize) -> impl IntoView {
    if count == 0 {
        return ().into_any();
    }
    view! { <ThreadCount txid=txid count=count /> }.into_any()
}
