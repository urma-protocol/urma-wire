use leptos::prelude::*;
use super::mock_data::Reply;
use wire_app_core::avatar::render_identicon_svg;

/// Variant A: Baseline V0 Reply Card
/// Left-bordered indented response card with target txid badge and confirmation metadata.
#[component]
pub fn ReplyVariantA(reply: Reply) -> impl IntoView {
    let author = reply.author;
    let author_fingerprint = reply.author.fingerprint();
    let txid_disp = reply.txid.to_display_hex();
    let txid_short = reply.txid.short_display();
    let target_short = reply.target_txid.short_display();
    let block_height = reply.status.block_height;
    let confirmations = reply.status.confirmations;
    let identicon = render_identicon_svg(&author, 32);

    view! {
        <article class="reply-card">
            <div class="post-header">
                <div class="author-link">
                    <div class="avatar-container" style="width: 32px; height: 32px;" inner_html=identicon />
                    <div class="author-meta">
                        <span class="author-name">"Elena Vance"</span>
                        <span class="author-fingerprint">"🔑 " {author_fingerprint}</span>
                    </div>
                </div>
                <div class="post-status-meta">
                    <span class="badge badge-target">"↩ to " {target_short}</span>
                    <span class="badge badge-block">"#" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <div class="markdown-content">{reply.text.clone()}</div>
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>
            </div>
        </article>
    }
}

/// Variant B: Lineage Tree & Parent Context Card
/// Renders an explicit parent context quote preview with SVG thread line connector.
#[component]
pub fn ReplyVariantB(reply: Reply) -> impl IntoView {
    let author = reply.author;
    let author_fingerprint = reply.author.fingerprint();
    let target_short = reply.target_txid.short_display();
    let identicon = render_identicon_svg(&author, 28);

    view! {
        <div class="reply-lineage-wrapper">
            <div class="lineage-stem">
                <svg class="lineage-branch-svg" width="24" height="60" viewBox="0 0 24 60" fill="none">
                    <path d="M12 0 v36 q0 12 12 12" stroke="#43523b" stroke-width="2" fill="none" />
                </svg>
            </div>

            <article class="reply-card-tree">
                <div class="parent-context-bar">
                    <span class="parent-arrow">"↳ In response to dispatch"</span>
                    <code class="parent-txid">"tx: " {target_short}</code>
                    <span class="parent-preview-hint">"· [Satellite Imagery Analysis]"</span>
                </div>

                <div class="tree-header">
                    <div class="author-link">
                        <div class="avatar-container" style="width: 28px; height: 28px;" inner_html=identicon />
                        <span class="tree-author-name">"Elena Vance"</span>
                        <span class="tree-author-fp">"[" {author_fingerprint} "]"</span>
                    </div>
                    <div class="tree-badges">
                        <span class="badge badge-tree-status">"Block #" {reply.status.block_height}</span>
                        <span class="badge badge-conf">{reply.status.confirmations} "c"</span>
                    </div>
                </div>

                <div class="tree-content">
                    <p>{reply.text.clone()}</p>
                </div>

                <div class="tree-footer">
                    <span class="proof-verified-mini">"✓ Schnorr Signed"</span>
                    <code class="tree-txid">"tx: " {reply.txid.short_display()}</code>
                </div>
            </article>
        </div>
    }
}

/// Variant C: Micro Dialogue Bubble
/// Compact conversational bubble optimized for rapid back-and-forth fact checking.
#[component]
pub fn ReplyVariantC(reply: Reply) -> impl IntoView {
    let author = reply.author;
    let author_fingerprint = reply.author.fingerprint();
    let identicon = render_identicon_svg(&author, 24);

    view! {
        <div class="reply-bubble-container">
            <div class="bubble-avatar">
                <div class="avatar-container" style="width: 24px; height: 24px;" inner_html=identicon />
            </div>
            <div class="reply-bubble-body">
                <div class="bubble-header">
                    <strong class="bubble-author">"Elena Vance"</strong>
                    <span class="bubble-fp">"@" {author_fingerprint}</span>
                    <span class="bubble-target">"↩" {reply.target_txid.short_display()}</span>
                    <span class="bubble-conf">{reply.status.confirmations} " conf"</span>
                </div>
                <div class="bubble-text">
                    {reply.text.clone()}
                </div>
                <div class="bubble-meta">
                    <code>{reply.txid.short_display()}</code>
                </div>
            </div>
        </div>
    }
}
