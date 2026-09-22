use leptos::prelude::*;
use wire_app_shared::{render_identicon_svg, Post};

/// Variant A: Baseline V0 Post Card
/// Standard URMA Wire feed card with author meta, block status, topic labels, and reply count.
#[component]
pub fn PostVariantA(post: Post) -> impl IntoView {
    let author = post.author;
    let author_hex = author.to_hex();
    let author_fingerprint = author.fingerprint();
    let txid_disp = post.txid.to_display_hex();
    let txid_short = post.txid.short_display();
    let reply_count = post.reply_count;
    let block_height = post.status.block_height;
    let confirmations = post.status.confirmations;

    let identicon = render_identicon_svg(&author, 40);

    view! {
        <article class="post-card">
            <div class="post-header">
                <div class="author-link">
                    <div class="avatar-container" style="width: 40px; height: 40px;" inner_html=identicon />
                    <div class="author-meta">
                        <span class="author-name">"Alex Rivera"</span>
                        <span class="author-fingerprint" title=format!("BIP 340 x-only public key: {author_hex}")>
                            {author_fingerprint}
                        </span>
                    </div>
                </div>
                <div class="post-status-meta">
                    <span class="badge badge-block">"#" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} " conf"</span>
                </div>
            </div>

            <div class="post-content">
                <p class="topic-labels">
                    {(!post.wire.is_empty()).then(|| format!("/{}", post.wire))}
                    " "
                    {post.hashtags.iter().map(|tag| format!("#{tag}")).collect::<Vec<_>>().join(" ")}
                </p>
                <div class="markdown-content">{post.text.clone()}</div>
            </div>

            <div class="post-footer">
                <span class="post-txid" title=format!("Reveal TxID: {txid_disp}")>
                    "tx: " {txid_short}
                </span>

                <div class="post-actions">
                    <span class="reply-btn">
                        {reply_count} " " {if reply_count == 1 { "reply" } else { "replies" }} " ↗"
                    </span>
                </div>
            </div>
        </article>
    }
}

/// Variant B: Compact Terminal Row
/// High-density feed item designed for OSINT analysts and real-time monitoring streams.
#[component]
pub fn PostVariantB(post: Post) -> impl IntoView {
    let author = post.author;
    let author_fingerprint = author.fingerprint();
    let txid_short = post.txid.short_display();
    let block_height = post.status.block_height;
    let confirmations = post.status.confirmations;
    let identicon = render_identicon_svg(&author, 24);
    let expanded = RwSignal::new(false);

    view! {
        <article class="post-card-compact">
            <div class="compact-header-row">
                <div class="compact-left">
                    <div class="avatar-container" style="width: 24px; height: 24px;" inner_html=identicon />
                    <span class="compact-author">"Alex Rivera"</span>
                    <span class="compact-fingerprint">"🔑 " {author_fingerprint}</span>
                    {(!post.wire.is_empty()).then(|| view! {
                        <span class="compact-wire-pill">"/" {post.wire.clone()}</span>
                    })}
                </div>
                <div class="compact-right">
                    <span class="badge badge-block">"#" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} "c"</span>
                    <span class="compact-txid">"tx: " {txid_short}</span>
                </div>
            </div>

            <div class="compact-body" on:click=move |_| expanded.update(|e| *e = !*e)>
                <p class=move || if expanded.get() { "compact-text expanded" } else { "compact-text clamped" }>
                    {post.text.clone()}
                </p>
            </div>

            <div class="compact-footer-row">
                <div class="compact-tags">
                    {post.hashtags.iter().map(|tag| view! {
                        <span class="compact-tag">"#" {tag.clone()}</span>
                    }).collect_view()}
                </div>
                <div class="compact-actions">
                    <button type="button" class="compact-action-btn" on:click=move |_| expanded.update(|e| *e = !*e)>
                        {move || if expanded.get() { "Collapse ▲" } else { "Expand ▼" }}
                    </button>
                    <span class="compact-reply-badge">{post.reply_count} " replies"</span>
                </div>
            </div>
        </article>
    }
}

/// Variant C: Investigative & Forensic Proof Card
/// Promotes cryptographic authenticity, displaying BIP-340 verification seal, Merkle root, and tamper-evident audit badge.
#[component]
pub fn PostVariantC(post: Post) -> impl IntoView {
    let author = post.author;
    let author_hex = author.to_hex();
    let author_fingerprint = author.fingerprint();
    let txid_disp = post.txid.to_display_hex();
    let block_height = post.status.block_height;
    let confirmations = post.status.confirmations;
    let identicon = render_identicon_svg(&author, 36);
    let copied = RwSignal::new(false);

    let copy_proof = move |_| {
        copied.set(true);
        // Reset feedback after 2s
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            copied.set(false);
        });
    };

    view! {
        <article class="post-card-investigative">
            <div class="investigative-top-rail">
                <div class="proof-seal">
                    <span class="proof-icon">"🛡️"</span>
                    <span class="proof-label">"BIP-340 VERIFIED RECORD"</span>
                    <span class="proof-dot"></span>
                    <span class="proof-chain">"LTC TESTNET"</span>
                </div>
                <div class="proof-block-info">
                    <span class="badge badge-proof-block">"Block #" {block_height}</span>
                    <span class="badge badge-conf">{confirmations} " Confirmed"</span>
                </div>
            </div>

            <div class="investigative-author-bar">
                <div class="avatar-container" style="width: 36px; height: 36px;" inner_html=identicon />
                <div class="investigative-author-info">
                    <div class="author-primary">
                        <strong class="author-title">"Alex Rivera"</strong>
                        <span class="author-org">"[OSINT Analyst · Corroborated]"</span>
                    </div>
                    <code class="bip340-key-mono" title=author_hex.clone()>
                        "pubkey: " {author.short_display()} " (" {author_fingerprint} ")"
                    </code>
                </div>
                <div class="wire-channel-tag">
                    <span class="wire-symbol">"📡"</span>
                    <span class="wire-name">"/" {post.wire.clone()}</span>
                </div>
            </div>

            <div class="investigative-body">
                <div class="markdown-content">{post.text.clone()}</div>
            </div>

            <div class="investigative-evidence-box">
                <div class="evidence-header">
                    <span class="evidence-title">"Cryptographic Anchor Evidence"</span>
                    <span class="evidence-status">"Consensus Validated"</span>
                </div>
                <div class="evidence-grid">
                    <div>
                        <span class="evidence-lbl">"Reveal TxID:"</span>
                        <code class="evidence-val">{txid_disp}</code>
                    </div>
                    <div>
                        <span class="evidence-lbl">"Block Commitment:"</span>
                        <code class="evidence-val">{post.status.block_hash.clone()}</code>
                    </div>
                </div>
            </div>

            <div class="investigative-footer">
                <div class="investigative-tags">
                    {post.hashtags.iter().map(|tag| view! {
                        <span class="forensic-tag">"#" {tag.clone()}</span>
                    }).collect_view()}
                </div>
                <div class="investigative-actions">
                    <button type="button" class="btn-forensic-copy" on:click=copy_proof>
                        {move || if copied.get() { "✓ Proof Copied!" } else { "📋 Copy Proof Bundle" }}
                    </button>
                    <span class="reply-count-tag">{post.reply_count} " Dispatches ↗"</span>
                </div>
            </div>
        </article>
    }
}

/// Variant D: Editorial / Longform Dispatch
/// Clean, publication-focused design with an accent left rail, spacious typography, and minimal visual noise.
#[component]
pub fn PostVariantD(post: Post) -> impl IntoView {
    let author = post.author;
    let author_fingerprint = author.fingerprint();
    let identicon = render_identicon_svg(&author, 32);

    view! {
        <article class="post-card-editorial">
            <header class="editorial-header">
                <div class="editorial-meta-left">
                    <div class="avatar-container" style="width: 32px; height: 32px;" inner_html=identicon />
                    <div class="editorial-byline">
                        <span class="editorial-author">"Alex Rivera"</span>
                        <span class="editorial-sub">"Sovereign Reporter · " {author_fingerprint}</span>
                    </div>
                </div>
                <div class="editorial-channel">
                    <span class="channel-dot"></span>
                    <span class="channel-slug">"/" {post.wire.clone()}</span>
                </div>
            </header>

            <div class="editorial-content">
                <div class="editorial-lead-quote">
                    <p>{post.text.clone()}</p>
                </div>
            </div>

            <footer class="editorial-footer">
                <div class="editorial-topics">
                    {post.hashtags.iter().map(|t| view! {
                        <span class="editorial-topic-pill">"#" {t.clone()}</span>
                    }).collect_view()}
                </div>
                <div class="editorial-stats">
                    <span class="stat-conf">"Block " {post.status.block_height} " (" {post.status.confirmations} " conf)"</span>
                    <span class="stat-replies">{post.reply_count} " responses"</span>
                </div>
            </footer>
        </article>
    }
}
