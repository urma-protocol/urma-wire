use super::mock_data::{mock_author_auditor, mock_author_investigator, mock_author_source};
use leptos::prelude::*;
use wire_app_core::avatar::render_identicon_svg;

/// Variant A: Wire Channel Dossier Banner
/// Header card for structured wires like `/rosint` showing provenance, stats, active contributors, and local follow toggle.
#[component]
pub fn WireDossierCard() -> impl IntoView {
    let following = RwSignal::new(true);
    let (a1, _, _) = mock_author_investigator();
    let (a2, _, _) = mock_author_auditor();
    let (a3, _, _) = mock_author_source();

    let id1 = render_identicon_svg(&a1, 24);
    let id2 = render_identicon_svg(&a2, 24);
    let id3 = render_identicon_svg(&a3, 24);

    view! {
        <div class="wire-dossier-banner">
            <div class="dossier-top-line">
                <div class="wire-brand-title">
                    <span class="wire-icon">"📡"</span>
                    <h2 class="wire-name">"/rosint"</h2>
                    <span class="wire-type-badge">"Open Protocol Channel"</span>
                </div>
                <div class="wire-follow-action">
                    <button
                        type="button"
                        class=move || if following.get() { "btn btn-secondary btn-sm active-follow" } else { "btn btn-primary btn-sm" }
                        on:click=move |_| following.update(|f| *f = !*f)
                    >
                        {move || if following.get() { "✓ Following Wire" } else { "+ Follow Wire Locally" }}
                    </button>
                </div>
            </div>

            <p class="wire-mission-text">
                "Open open-source intelligence wire for geospatial reconnaissance, maritime vessel tracking, and verifiable satellite sensor corroboration."
            </p>

            <div class="wire-meta-bar">
                <div class="wire-stat-item">
                    <span class="stat-num">"1,420"</span>
                    <span class="stat-desc">"Anchored Records"</span>
                </div>
                <div class="wire-stat-item">
                    <span class="stat-num">"38"</span>
                    <span class="stat-desc">"Active Analysts"</span>
                </div>
                <div class="wire-stat-item">
                    <span class="stat-num">"0"</span>
                    <span class="stat-desc">"Central Servers"</span>
                </div>
                <div class="wire-contributors-group">
                    <span class="contrib-label">"Top Keyholders:"</span>
                    <div class="contrib-avatars">
                        <div class="avatar-container" style="width: 24px; height: 24px;" inner_html=id1 />
                        <div class="avatar-container" style="width: 24px; height: 24px;" inner_html=id2 />
                        <div class="avatar-container" style="width: 24px; height: 24px;" inner_html=id3 />
                    </div>
                </div>
            </div>

            <div class="wire-topics-row">
                <span class="topics-label">"Channel Keywords:"</span>
                <span class="topic-chip">"#investigation"</span>
                <span class="topic-chip">"#satellites"</span>
                <span class="topic-chip">"#maritime"</span>
                <span class="topic-chip">"#ais"</span>
                <span class="topic-chip">"#signals"</span>
            </div>
        </div>
    }
}

/// Variant B: Chain Proof Inspector Card
/// Visual cryptographic audit tool for deep forensic verification of an on-chain record.
#[component]
pub fn ProofInspectorCard() -> impl IntoView {
    let inspect_tab = RwSignal::new("merkle");

    view! {
        <div class="proof-inspector-card">
            <div class="inspector-header">
                <div class="inspector-title">
                    <span class="inspector-badge">"FORENSIC PROOF INSPECTOR"</span>
                    <h4>"Record tx: 9f8a774c...33d1"</h4>
                </div>
                <div class="inspector-status-pill">
                    <span class="valid-dot"></span>
                    <strong class="valid-text">"CRYPTOGRAPHICALLY SOUND"</strong>
                </div>
            </div>

            <div class="inspector-tabs">
                <button
                    type="button"
                    class=move || if inspect_tab.get() == "merkle" { "tab-btn active" } else { "tab-btn" }
                    on:click=move |_| inspect_tab.set("merkle")
                >
                    "Merkle Tree Path"
                </button>
                <button
                    type="button"
                    class=move || if inspect_tab.get() == "schnorr" { "tab-btn active" } else { "tab-btn" }
                    on:click=move |_| inspect_tab.set("schnorr")
                >
                    "BIP-340 Schnorr"
                </button>
                <button
                    type="button"
                    class=move || if inspect_tab.get() == "script" { "tab-btn active" } else { "tab-btn" }
                    on:click=move |_| inspect_tab.set("script")
                >
                    "Witness Script"
                </button>
            </div>

            <div class="inspector-body">
                {move || match inspect_tab.get() {
                    "merkle" => view! {
                        <div class="inspector-tab-content">
                            <div class="proof-row">
                                <span class="proof-label">"Root Checkpoint:"</span>
                                <code class="proof-val">"3a8f10bc92d8471e55c0299aa14bc8fe9210c4d8123019847182746190abcf12"</code>
                            </div>
                            <div class="proof-row">
                                <span class="proof-label">"Transaction Index:"</span>
                                <span class="proof-val">"Leaf #8 in Block 4,892,707"</span>
                            </div>
                            <div class="proof-row">
                                <span class="proof-label">"Sibling Hash L3:"</span>
                                <code class="proof-val">"c8109d43120ea...551029ba"</code>
                            </div>
                        </div>
                    }.into_any(),
                    "schnorr" => view! {
                        <div class="inspector-tab-content">
                            <div class="proof-row">
                                <span class="proof-label">"Author Public Key:"</span>
                                <code class="proof-val">"3b8d4f1a0000000000000000000000000000000000000000000000000000009f"</code>
                            </div>
                            <div class="proof-row">
                                <span class="proof-label">"Signature Status:"</span>
                                <span class="proof-val proof-pass">"✓ VALID (secp256k1 BIP 340 verification passed)"</span>
                            </div>
                            <div class="proof-row">
                                <span class="proof-label">"Replay Protection:"</span>
                                <span class="proof-val">"Commit tx funding outpoint verified unspent"</span>
                            </div>
                        </div>
                    }.into_any(),
                    _ => view! {
                        <div class="inspector-tab-content">
                            <div class="proof-row">
                                <span class="proof-label">"Witness Script Envelope:"</span>
                                <code class="proof-val">"OP_0 OP_IF 0x75726d61 [payload bytes: 241] OP_ENDIF"</code>
                            </div>
                            <div class="proof-row">
                                <span class="proof-label">"Kind Byte:"</span>
                                <span class="proof-val">"0x0A (Structured Wire Post)"</span>
                            </div>
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}

/// Variant C: Block Checkpoint Timeline Node
/// Shows verifiable chain ingestion checkpoints and rollback resilience indicators.
#[component]
pub fn BlockCheckpointNode() -> impl IntoView {
    view! {
        <div class="checkpoint-timeline-node">
            <div class="node-marker">
                <span class="marker-dot"></span>
                <span class="marker-line"></span>
            </div>
            <div class="node-content">
                <div class="node-header">
                    <span class="node-title">"Block #4,892,707 Checkpoint"</span>
                    <span class="badge badge-conf">"42 Confirmations"</span>
                    <span class="badge badge-block">"Format-2 Index"</span>
                </div>
                <p class="node-hash">
                    "Hash: " <code>"3a8f10bc92d8471e55c0299aa14bc8fe9210c4d8123019847182746190abcf12"</code>
                </p>
                <div class="node-metrics-bar">
                    <span class="node-metric">"Indexed Records: <strong>14</strong>"</span>
                    <span class="node-metric">"URMA Authors: <strong>4</strong>"</span>
                    <span class="node-metric">"Rollback Status: <strong>Canonical Tip</strong>"</span>
                </div>
            </div>
        </div>
    }
}
