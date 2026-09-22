pub mod composer;
pub mod identity;
pub mod investigation;
pub mod media;
pub mod metrics;
pub mod mock_data;
pub mod posts;
pub mod publication;
pub mod replies;

use leptos::prelude::*;
use mock_data::{
    mock_post_editorial, mock_post_investigative, mock_reply_corroboration,
    mock_reply_quick,
};

#[component]
pub fn ComponentsPage() -> impl IntoView {
    let active_category = RwSignal::new("all".to_string());
    let view_mode = RwSignal::new("stacked".to_string()); // "stacked" or "grid"

    let categories = vec![
        ("all", "All Components", "24"),
        ("posts", "Feed & Posts", "4"),
        ("replies", "Replies & Threads", "3"),
        ("composer", "Composer & Dispatch", "3"),
        ("identity", "Identity & Avatars", "5"),
        ("publication", "Publication & Fees", "3"),
        ("media", "Media & Evidence", "2"),
        ("investigation", "OSINT & Wires", "3"),
        ("metrics", "Widgets & Metrics", "2"),
    ];

    view! {
        <section class="page-container gallery-page">
            <header class="gallery-hero">
                <span class="eyebrow">"DESIGN SYSTEM & EXPLORATION LAB"</span>
                <h1 class="page-title">"UI Components Gallery"</h1>
                <p class="page-subtitle">
                    "Interactive showcase for evaluating and comparing UI components for URMA Wire. "
                    "Compare layout density, cryptographic proof depth, and future OSINT features before choosing what to adopt."
                </p>

                <div class="gallery-controls-bar">
                    <div class="category-tabs" role="tablist">
                        {categories.into_iter().map(|(id, label, count)| {
                            let is_active = move || active_category.get() == id;
                            view! {
                                <button
                                    type="button"
                                    class=move || if is_active() { "cat-tab active" } else { "cat-tab" }
                                    on:click=move |_| active_category.set(id.to_string())
                                >
                                    <span>{label}</span>
                                    <span class="cat-count">{count}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>

                    <div class="view-mode-toggle">
                        <button
                            type="button"
                            class=move || if view_mode.get() == "stacked" { "mode-btn active" } else { "mode-btn" }
                            on:click=move |_| view_mode.set("stacked".to_string())
                            title="Stacked view"
                        >
                            "Stacked"
                        </button>
                        <button
                            type="button"
                            class=move || if view_mode.get() == "grid" { "mode-btn active" } else { "mode-btn" }
                            on:click=move |_| view_mode.set("grid".to_string())
                            title="Grid comparison view"
                        >
                            "Grid"
                        </button>
                    </div>
                </div>
            </header>

            // Category 1: Posts & Feed Cards
            {move || (active_category.get() == "all" || active_category.get() == "posts").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 01"</span>
                            <h2>"Feed & Post Cards"</h2>
                            <p>"Four distinct approaches to presenting on-chain dispatches with varying emphasis on metadata, density, and cryptographic auditability."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        // Variant A
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-baseline">"Variant A · Baseline V0"</span>
                                    <span class="v-tag">"Current Production"</span>
                                </div>
                                <h3 class="variant-title">"Standard Social Card"</h3>
                                <p class="variant-desc">"Default layout balancing author meta, topic tags, reply counters, and reveal TxID."</p>
                                <span class="variant-code-ref">"Source: components_gallery/posts.rs::PostVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <posts::PostVariantA post=mock_post_investigative() />
                            </div>
                        </div>

                        // Variant B
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Proposed V1"</span>
                                    <span class="v-tag">"High-Density Terminal"</span>
                                </div>
                                <h3 class="variant-title">"Compact OSINT Stream Row"</h3>
                                <p class="variant-desc">"Condensed horizontal row with inline badges and expandable body. Maximizes records visible above the fold."</p>
                                <span class="variant-code-ref">"Source: components_gallery/posts.rs::PostVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <posts::PostVariantB post=mock_post_investigative() />
                            </div>
                        </div>

                        // Variant C
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant C · Proposed V1"</span>
                                    <span class="v-tag">"Forensic Investigation"</span>
                                </div>
                                <h3 class="variant-title">"Cryptographic Proof & Anchor Card"</h3>
                                <p class="variant-desc">"Emphasizes zero-trust verification: BIP-340 Schnorr signature seal, Merkle root commitment box, and 1-click proof bundle export."</p>
                                <span class="variant-code-ref">"Source: components_gallery/posts.rs::PostVariantC"</span>
                            </div>
                            <div class="variant-canvas">
                                <posts::PostVariantC post=mock_post_investigative() />
                            </div>
                        </div>

                        // Variant D
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant D · Exploratory"</span>
                                    <span class="v-tag">"Editorial Magazine"</span>
                                </div>
                                <h3 class="variant-title">"Editorial Longform Dispatch"</h3>
                                <p class="variant-desc">"Publication-style layout with colored vertical topic rail, large body typography, and subtle technical metadata."</p>
                                <span class="variant-code-ref">"Source: components_gallery/posts.rs::PostVariantD"</span>
                            </div>
                            <div class="variant-canvas">
                                <posts::PostVariantD post=mock_post_editorial() />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 2: Replies & Threads
            {move || (active_category.get() == "all" || active_category.get() == "replies").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 02"</span>
                            <h2>"Replies & Thread Lineage"</h2>
                            <p>"Three distinct visual models for threaded conversational hierarchy and corroboration dispatches."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        // Variant A
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-baseline">"Variant A · Baseline V0"</span>
                                    <span class="v-tag">"Current Production"</span>
                                </div>
                                <h3 class="variant-title">"Indented Card with Target TxID Badge"</h3>
                                <p class="variant-desc">"Green accent left border with ↩ target indicator and confirmations."</p>
                                <span class="variant-code-ref">"Source: components_gallery/replies.rs::ReplyVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <replies::ReplyVariantA reply=mock_reply_corroboration() />
                            </div>
                        </div>

                        // Variant B
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Proposed V1"</span>
                                    <span class="v-tag">"Thread Hierarchy"</span>
                                </div>
                                <h3 class="variant-title">"Lineage Tree & Parent Context Card"</h3>
                                <p class="variant-desc">"Displays an explicit parent summary quote connected with a tree branch SVG line."</p>
                                <span class="variant-code-ref">"Source: components_gallery/replies.rs::ReplyVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <replies::ReplyVariantB reply=mock_reply_corroboration() />
                            </div>
                        </div>

                        // Variant C
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant C · Exploratory"</span>
                                    <span class="v-tag">"Fast Discourse"</span>
                                </div>
                                <h3 class="variant-title">"Micro Dialogue Bubble"</h3>
                                <p class="variant-desc">"Lightweight speech-bubble styling designed for collaborative real-time fact checking."</p>
                                <span class="variant-code-ref">"Source: components_gallery/replies.rs::ReplyVariantC"</span>
                            </div>
                            <div class="variant-canvas">
                                <replies::ReplyVariantC reply=mock_reply_quick() />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 3: Composer & Publishing
            {move || (active_category.get() == "all" || active_category.get() == "composer").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 03"</span>
                            <h2>"Composer & Dispatch Interfaces"</h2>
                            <p>"Variants ranging from the compact 2-line input to an advanced split-screen live markdown studio and a quick broadcast bar."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-baseline">"Variant A · Baseline V0"</span>
                                    <span class="v-tag">"Current Production"</span>
                                </div>
                                <h3 class="variant-title">"Compact Accordion Composer"</h3>
                                <p class="variant-desc">"Collapsible wire & hashtag details with raw byte meter and review trigger."</p>
                                <span class="variant-code-ref">"Source: components_gallery/composer.rs::ComposerVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <composer::ComposerVariantA />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Proposed V1"</span>
                                    <span class="v-tag">"Advanced Journalist Studio"</span>
                                </div>
                                <h3 class="variant-title">"Split Studio & Live Preview Composer"</h3>
                                <p class="variant-desc">"Dual-pane markdown editor with instant preview, fee gauge, and UTF-8 meter bar."</p>
                                <span class="variant-code-ref">"Source: components_gallery/composer.rs::ComposerVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <composer::ComposerVariantB />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant C · Exploratory"</span>
                                    <span class="v-tag">"Breaking News"</span>
                                </div>
                                <h3 class="variant-title">"Quick Wire Dispatch Bar"</h3>
                                <p class="variant-desc">"Single-line broadcast bar with 1-click channel pills (/rosint, /intel, /alerts) and urgent flag."</p>
                                <span class="variant-code-ref">"Source: components_gallery/composer.rs::ComposerVariantC"</span>
                            </div>
                            <div class="variant-canvas">
                                <composer::ComposerVariantC />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 4: Identity & Avatars
            {move || (active_category.get() == "all" || active_category.get() == "identity").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 04"</span>
                            <h2>"Identity, Avatars & Keychips"</h2>
                            <p>"Representing cryptographic actors: from raw 16-color pixel drawings to verified BIP-340 key chips and comprehensive analyst dossiers."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-baseline">"Variant A · Baseline V0"</span>
                                    <span class="v-tag">"Standard Sizes"</span>
                                </div>
                                <h3 class="variant-title">"Pixel Avatar & Identicon Renderers"</h3>
                                <p class="variant-desc">"32x32 pixel matrix rendered via run-length encoded SVG alongside algorithmic identicons."</p>
                                <span class="variant-code-ref">"Source: components_gallery/identity.rs::IdentityVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <identity::IdentityVariantA />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Proposed V1"</span>
                                    <span class="v-tag">"Universal Badge"</span>
                                </div>
                                <h3 class="variant-title">"Cryptographic Identity Pill / Chip"</h3>
                                <p class="variant-desc">"Compact pill combining avatar, handle, key fingerprint, session status dot, and click-to-copy."</p>
                                <span class="variant-code-ref">"Source: components_gallery/identity.rs::IdentityVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <identity::IdentityVariantB />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant C · Proposed V1"</span>
                                    <span class="v-tag">"Profile Dossier"</span>
                                </div>
                                <h3 class="variant-title">"Forensic Investigator Dossier Card"</h3>
                                <p class="variant-desc">"Full profile card displaying 64-char x-only pubkey, indexing history, storage state, and vault export controls."</p>
                                <span class="variant-code-ref">"Source: components_gallery/identity.rs::IdentityVariantC"</span>
                            </div>
                            <div class="variant-canvas">
                                <identity::IdentityVariantC />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant D · Gallery"</span>
                                    <span class="v-tag">"16-Color Palette"</span>
                                </div>
                                <h3 class="variant-title">"Pixel-Art Avatar Matrix"</h3>
                                <p class="variant-desc">"Sample avatars created with the URMA 16-color palette demonstrating contrast and iconography."</p>
                                <span class="variant-code-ref">"Source: components_gallery/identity.rs::IdentityVariantD"</span>
                            </div>
                            <div class="variant-canvas">
                                <identity::IdentityVariantD />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant E · Interactive Studio"</span>
                                    <span class="v-tag">"Advanced Editor"</span>
                                </div>
                                <h3 class="variant-title">"Interactive Avatar Drawing Studio"</h3>
                                <p class="variant-desc">"Full drawing sandbox: Pencil (1px, 2x2, 3x3, Cross), Flood Fill Bucket, Eraser, Eyedropper, and complete history undo/redo (Ctrl+Z / Ctrl+Y)."</p>
                                <span class="variant-code-ref">"Source: components_gallery/identity.rs::IdentityVariantStudio"</span>
                            </div>
                            <div class="variant-canvas">
                                <identity::IdentityVariantStudio />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 5: Publication & Fee Approvals
            {move || (active_category.get() == "all" || active_category.get() == "publication").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 05"</span>
                            <h2>"Publication & Consensus Approval"</h2>
                            <p>"Presenting transaction fees, spendable outputs, and the atomic commit-reveal publication cycle."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-baseline">"Variant A · Baseline V0"</span>
                                    <span class="v-tag">"Current Production"</span>
                                </div>
                                <h3 class="variant-title">"3-Column Cost Grid Panel"</h3>
                                <p class="variant-desc">"Displays exact fee, funding selected, and change returned with a 2-stage progress bar."</p>
                                <span class="variant-code-ref">"Source: components_gallery/publication.rs::PublicationVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <publication::PublicationVariantA />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Proposed V1"</span>
                                    <span class="v-tag">"Step Tracker"</span>
                                </div>
                                <h3 class="variant-title">"Cryptographic Pipeline Ledger"</h3>
                                <p class="variant-desc">"5-stage visual stepper (Quote → Sign → Commit → Reveal → Final) with transaction hashes."</p>
                                <span class="variant-code-ref">"Source: components_gallery/publication.rs::PublicationVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <publication::PublicationVariantB />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-exploratory">"Variant C · Exploratory"</span>
                                    <span class="v-tag">"Streamlined"</span>
                                </div>
                                <h3 class="variant-title">"Streamlined Approval Drawer / Banner"</h3>
                                <p class="variant-desc">"Minimal banner keeping writers in flow with clear satoshi cost and 1-click approval."</p>
                                <span class="variant-code-ref">"Source: components_gallery/publication.rs::PublicationVariantC"</span>
                            </div>
                            <div class="variant-canvas">
                                <publication::PublicationVariantC />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 6: Media & Evidence Verification
            {move || (active_category.get() == "all" || active_category.get() == "media").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 06"</span>
                            <h2>"Media & Evidence Verification"</h2>
                            <p>"Components designed for the strict external image architecture documented in WIRE: unicast DNS pinning, SHA-256 fingerprinting, zero disk cache, and privacy quarantine."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant A · Future V1"</span>
                                    <span class="v-tag">"Security Verified"</span>
                                </div>
                                <h3 class="variant-title">"Inspected External Media Card"</h3>
                                <p class="variant-desc">"Displays verified satellite/photo evidence with public unicast IP pinning, SHA-256 hash, and memory purge control."</p>
                                <span class="variant-code-ref">"Source: components_gallery/media.rs::MediaVariantA"</span>
                            </div>
                            <div class="variant-canvas">
                                <media::MediaVariantA />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Future V1"</span>
                                    <span class="v-tag">"Privacy Quarantine"</span>
                                </div>
                                <h3 class="variant-title">"Privacy Shield & Quarantine Box"</h3>
                                <p class="variant-desc">"Shown when autoload is disabled or untrusted host referenced. Explains privacy risks before loading."</p>
                                <span class="variant-code-ref">"Source: components_gallery/media.rs::MediaVariantB"</span>
                            </div>
                            <div class="variant-canvas">
                                <media::MediaVariantB />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 7: OSINT & Wire Investigations
            {move || (active_category.get() == "all" || active_category.get() == "investigation").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 07"</span>
                            <h2>"OSINT & Wire Investigations"</h2>
                            <p>"Components for open protocol channels (/rosint, /intel), cryptographic Merkle path inspection, and block checkpoint timelines."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant A · Future V1"</span>
                                    <span class="v-tag">"Channel Banner"</span>
                                </div>
                                <h3 class="variant-title">"Wire Channel Dossier Banner"</h3>
                                <p class="variant-desc">"Channel header displaying open protocol notice, follow button, active analyst avatars, and keyword cloud."</p>
                                <span class="variant-code-ref">"Source: components_gallery/investigation.rs::WireDossierCard"</span>
                            </div>
                            <div class="variant-canvas">
                                <investigation::WireDossierCard />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Future V1"</span>
                                    <span class="v-tag">"Deep Audit"</span>
                                </div>
                                <h3 class="variant-title">"Forensic Chain Proof Inspector"</h3>
                                <p class="variant-desc">"Tabbed inspector for reviewing Merkle tree paths, BIP-340 Schnorr signature status, and witness script envelopes."</p>
                                <span class="variant-code-ref">"Source: components_gallery/investigation.rs::ProofInspectorCard"</span>
                            </div>
                            <div class="variant-canvas">
                                <investigation::ProofInspectorCard />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant C · Future V1"</span>
                                    <span class="v-tag">"Sync Checkpoint"</span>
                                </div>
                                <h3 class="variant-title">"Block Checkpoint Timeline Node"</h3>
                                <p class="variant-desc">"Timeline component illustrating recent-history sync checkpoints, block hashes, and rollback safety."</p>
                                <span class="variant-code-ref">"Source: components_gallery/investigation.rs::BlockCheckpointNode"</span>
                            </div>
                            <div class="variant-canvas">
                                <investigation::BlockCheckpointNode />
                            </div>
                        </div>
                    </div>
                </section>
            })}

            // Category 8: Widgets & Metrics
            {move || (active_category.get() == "all" || active_category.get() == "metrics").then(|| view! {
                <section class="gallery-category-section">
                    <div class="category-header">
                        <div class="cat-title-block">
                            <span class="cat-eyebrow">"CATEGORY 08"</span>
                            <h2>"Widgets & Status Metrics"</h2>
                            <p>"Modern status widgets for chain connectivity, sync tip progress, and spendable funding capacity."</p>
                        </div>
                    </div>

                    <div class=move || if view_mode.get() == "grid" { "gallery-grid" } else { "gallery-stacked" }>
                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant A · Widget"</span>
                                    <span class="v-tag">"Consensus Tile"</span>
                                </div>
                                <h3 class="variant-title">"Network Consensus Status Tile"</h3>
                                <p class="variant-desc">"Live tip height, sync progress, and provider latency indicator."</p>
                                <span class="variant-code-ref">"Source: components_gallery/metrics.rs::NetworkMetricCard"</span>
                            </div>
                            <div class="variant-canvas">
                                <metrics::NetworkMetricCard />
                            </div>
                        </div>

                        <div class="variant-showcase">
                            <div class="variant-meta-header">
                                <div class="variant-tags">
                                    <span class="v-pill v-proposal">"Variant B · Widget"</span>
                                    <span class="v-tag">"Wallet Readiness"</span>
                                </div>
                                <h3 class="variant-title">"Spendable Funding Gauge"</h3>
                                <p class="variant-desc">"UTXO readiness, spendable satoshis, and estimated publication capacity."</p>
                                <span class="variant-code-ref">"Source: components_gallery/metrics.rs::SpendableGaugeWidget"</span>
                            </div>
                            <div class="variant-canvas">
                                <metrics::SpendableGaugeWidget />
                            </div>
                        </div>
                    </div>
                </section>
            })}
        </section>
    }
}
