use crate::components::markdown::Markdown;
use crate::components::AvatarView;
use crate::limits::CARD_AVATAR_PIXELS;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::{component, view};
use wire_app_shared::Post;

#[component]
pub fn PostCard(post: Post) -> impl IntoView {
    let state = expect_context::<AppState>();
    let author = post.author;
    let author_hex = author.to_hex();
    let txid_disp = post.txid.to_display_hex();
    let txid_short = post.txid.short_display();
    let reply_count = post.reply_count;
    let replying = post.clone();

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
                    <span class="badge badge-block">"#" {post.status.block_height}</span>
                    <span class="badge badge-conf">{post.status.confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <TopicLabels wire=post.wire.clone() hashtags=post.hashtags.clone() />
                <Markdown text=post.text.clone() />
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>
                <div class="post-actions">
                    <button type="button" class="post-action"
                        on:click=move |_clicked| state.reply_to(replying.clone())>
                        "↳ Reply"
                    </button>
                    <a href=format!("/post/{txid_disp}") class="post-action">
                        {reply_count}" "{plural(reply_count)}
                    </a>
                </div>
            </div>
        </article>
    }
}

fn plural(count: usize) -> &'static str {
    match count {
        1 => "reply",
        _ => "replies",
    }
}

#[component]
fn TopicLabels(wire: String, hashtags: Vec<String>) -> impl IntoView {
    let state = expect_context::<AppState>();
    if wire.is_empty() && hashtags.is_empty() {
        return ().into_any();
    }
    let mut spelled: Vec<String> = Vec::new();
    if !wire.is_empty() {
        spelled.push(format!("/{wire}"));
    }
    for tag in hashtags {
        spelled.push(format!("#{tag}"));
    }
    view! {
        <p class="topic-labels">
            {spelled.into_iter().map(|label| {
                let held = label.clone();
                view! {
                    <button type="button"
                        class=move || match state.topics.get().follows(&held) { true => "topic-chip followed", false => "topic-chip" }
                        title="Follow or unfollow this topic"
                        on:click={
                            let spelled = label.clone();
                            move |_clicked| state.toggle_follow(spelled.clone())
                        }>
                        {label.clone()}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </p>
    }.into_any()
}
