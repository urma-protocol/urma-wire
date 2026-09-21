use crate::components::AvatarView;
use leptos::prelude::*;
use wire_app_shared::Reply;

#[component]
pub fn ReplyCard(reply: Reply) -> impl IntoView {
    let author = reply.author;
    let author_hex = author.to_hex();
    let author_fingerprint = author.fingerprint();
    let state = expect_context::<crate::state::AppState>();
    let display_name = move || state.display_name(author);

    let txid_disp = reply.txid.to_display_hex();
    let txid_short = reply.txid.short_display();
    let target_short = reply.target_txid.short_display();
    let profile_url = format!("/profile/{author_hex}");
    let block_height = reply.status.block_height;
    let confirmations = reply.status.confirmations;

    view! {
        <article class="reply-card">
            <div class="post-header">
                <a href=profile_url class="author-link">
                    <AvatarView author=author size=32 />
                    <div class="author-meta">
                        <span class="author-name">{display_name}</span>
                        <span class="author-fingerprint">"🔑 " {author_fingerprint}</span>
                    </div>
                </a>
                <div class="post-status-meta">
                    <span class="badge badge-target">"↩ to " {target_short}</span>
                    <span class="badge badge-block">"#" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <super::markdown::Markdown text=reply.text.clone() />
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>
            </div>
        </article>
    }
}
