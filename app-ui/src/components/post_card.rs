use crate::components::markdown::PostText;
use crate::components::post_parts::{InlineReply, ReplyToggle, ThreadCount};
use crate::components::AvatarView;
use crate::config::CARD_AVATAR_PIXELS;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::{component, view};
use wire_app_core::model::{Placement, Post};

#[component]
pub fn PostCard(post: Post) -> impl IntoView {
    let state = expect_context::<AppState>();
    let author = post.author;
    let author_hex = author.to_hex();
    let txid = post.txid;
    let txid_disp = txid.to_display_hex();
    let txid_short = txid.short_display();
    let reply_count = post.reply_count;
    let block_height = post.status.block_height;
    let confirmations = post.status.confirmations;

    view! {
        <article class="post-card">
            <div class="post-header">
                <a href=format!("/profile/{author_hex}") class="author-link">
                    <AvatarView author=author size=CARD_AVATAR_PIXELS />
                    <div class="author-meta">
                        <span class="author-name">{move || state.display_name(author)}</span>
                        <span class="author-fingerprint" title=format!("BIP 340 x-only public key: {author_hex}")>
                            {author.fingerprint()}
                        </span>
                    </div>
                </a>
                <div class="post-status-meta">
                    <span class="badge" title="Block height">"#" {block_height}</span>
                    <span class="badge badge-conf" title="Confirmations">{confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <Answering placement=post.placement />
                <PostText text=post.text.clone() wire=post.wire.clone() hashtags=post.hashtags.clone() />
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>
                <div class="post-actions">
                    <ReplyToggle post=post.clone() />
                    <ThreadCount txid=txid count=reply_count />
                </div>
            </div>
            <InlineReply post=post />
        </article>
    }
}

#[component]
fn Answering(placement: Placement) -> impl IntoView {
    let Placement::Reply(parent) = placement else {
        return ().into_any();
    };
    let parent_disp = parent.to_display_hex();
    view! {
        <p class="post-answering">
            "↳ Replying to "
            <a href=format!("/post/{parent_disp}") title=format!("Parent TxID: {parent_disp}")>
                {parent.short_display()}
            </a>
        </p>
    }
    .into_any()
}
