use crate::components::AvatarView;
use leptos::prelude::*;
use wire_app_shared::Post;

#[component]
pub fn PostCard(post: Post) -> impl IntoView {
    let author = post.author;
    let author_hex = author.to_hex();
    let author_fingerprint = author.fingerprint();
    let state = expect_context::<crate::state::AppState>();
    let display_name = move || state.display_name(author);

    let txid_disp = post.txid.to_display_hex();
    let txid_short = post.txid.short_display();
    let post_url = format!("/post/{txid_disp}");
    let profile_url = format!("/profile/{author_hex}");
    let reply_count = post.reply_count;
    let block_height = post.status.block_height;
    let confirmations = post.status.confirmations;

    view! {
        <article class="post-card">
            <div class="post-header">
                <a href=profile_url.clone() class="author-link">
                    <AvatarView author=author size=40 />
                    <div class="author-meta">
                        <span class="author-name">{display_name}</span>
                        <span class="author-fingerprint" title=format!("BIP 340 x-only public key: {author_hex}")>
                            {author_fingerprint}
                        </span>
                    </div>
                </a>
                <div class="post-status-meta">
                    <span class="badge badge-block">"#" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <p class="topic-labels">{(!post.wire.is_empty()).then(|| format!("/{}", post.wire))} " " {post.hashtags.iter().map(|tag| format!("#{tag}")).collect::<Vec<_>>().join(" ")}</p>
                <super::markdown::Markdown text=post.text.clone() />
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>

                <div class="post-actions">
                    <a href=post_url class="reply-btn">
                        {reply_count} " " {if reply_count == 1 { "reply" } else { "replies" }} " ↗"
                    </a>
                </div>
            </div>
        </article>
    }
}
