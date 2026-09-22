use leptos::prelude::*;

/// Variant A: Baseline V0 Publication Review Card
/// 3-column cost grid with progress bar, transaction disclosure, and approval flow.
#[component]
pub fn PublicationVariantA() -> impl IntoView {
    view! {
        <section class="compose-card publication-card">
            <div class="compose-header">
                <strong>"Your publication plan · Post / reply"</strong>
                <span class="chain-badge-small">"LTC TESTNET"</span>
            </div>

            <p class="publication-text">
                "Initial satellite imagery corroborates naval repositioning off the Constanța corridor (44.128° N, 28.654° E)."
            </p>

            <div class="publication-costs">
                <div>
                    <span>"Exact total fee"</span>
                    <strong>"241 sats"</strong>
                    <small>"1 litoshi/vbyte rate"</small>
                </div>
                <div>
                    <span>"Selected funding"</span>
                    <strong>"100,000 sats"</strong>
                    <small>"1 confirmed output"</small>
                </div>
                <div>
                    <span>"Change returned"</span>
                    <strong>"99,759 sats"</strong>
                    <small>"Signed back to vault"</small>
                </div>
            </div>

            <div class="publication-status">
                <span>"Status: " <strong>"Ready for approval"</strong></span>
                <span>"Progress: 0 of 2 transactions confirmed"</span>
            </div>

            <progress class="publication-progress" value="0.0" max="1.0" />

            <div class="compose-buttons">
                <button type="button" class="btn btn-primary btn-sm">
                    "Approve fee & publish"
                </button>
                <button type="button" class="btn btn-secondary btn-sm">
                    "Pause publication"
                </button>
            </div>
        </section>
    }
}

/// Variant B: Cryptographic Pipeline Ledger
/// Visual 5-stage blockchain transaction progression showing exact cryptographic pipeline state.
#[component]
pub fn PublicationVariantB() -> impl IntoView {
    view! {
        <section class="compose-card pipeline-ledger-card">
            <div class="pipeline-header">
                <div class="pipeline-title-group">
                    <span class="eyebrow">"CRYPTOGRAPHIC CONSENSUS LIFECYCLE"</span>
                    <h4 class="pipeline-title">"Publication Pipeline #URMA-8921"</h4>
                </div>
                <span class="badge badge-conf">"Commitment Verified"</span>
            </div>

            <div class="pipeline-stepper">
                <div class="pipeline-step completed">
                    <div class="step-indicator">"✓"</div>
                    <div class="step-body">
                        <span class="step-title">"1. Exact Quote"</span>
                        <span class="step-sub">"241 sats · 1 sat/vB"</span>
                    </div>
                </div>
                <div class="step-connector completed"></div>

                <div class="pipeline-step completed">
                    <div class="step-indicator">"✓"</div>
                    <div class="step-body">
                        <span class="step-title">"2. BIP-340 Sign"</span>
                        <span class="step-sub">"Vault unlocked"</span>
                    </div>
                </div>
                <div class="step-connector active"></div>

                <div class="pipeline-step active">
                    <div class="step-indicator">"●"</div>
                    <div class="step-body">
                        <span class="step-title">"3. Commit Tx"</span>
                        <span class="step-sub">"Mempool observed"</span>
                    </div>
                </div>
                <div class="step-connector pending"></div>

                <div class="pipeline-step pending">
                    <div class="step-indicator">"○"</div>
                    <div class="step-body">
                        <span class="step-title">"4. Reveal Tx"</span>
                        <span class="step-sub">"Witness ingestion"</span>
                    </div>
                </div>
                <div class="step-connector pending"></div>

                <div class="pipeline-step pending">
                    <div class="step-indicator">"○"</div>
                    <div class="step-body">
                        <span class="step-title">"5. Block Final"</span>
                        <span class="step-sub">"Immutable feed"</span>
                    </div>
                </div>
            </div>

            <div class="pipeline-ledger-grid">
                <div class="ledger-box">
                    <span class="ledger-label">"Commit TxID (Mempool)"</span>
                    <code class="ledger-hash">"48f1a0...99c2"</code>
                </div>
                <div class="ledger-box">
                    <span class="ledger-label">"Reveal Payload Size"</span>
                    <strong class="ledger-val">"241 bytes UTF-8"</strong>
                </div>
                <div class="ledger-box">
                    <span class="ledger-label">"Network Settlement"</span>
                    <strong class="ledger-val">"Litecoin Testnet"</strong>
                </div>
            </div>

            <div class="pipeline-actions">
                <button type="button" class="btn btn-primary btn-sm">"Broadcast Reveal Tx ↗"</button>
                <button type="button" class="btn btn-secondary btn-sm">"Inspect Raw Signed Bytes"</button>
            </div>
        </section>
    }
}

/// Variant C: Streamlined Approval Drawer / Banner
/// Compact, low-profile approval strip designed for uninterrupted writing flow.
#[component]
pub fn PublicationVariantC() -> impl IntoView {
    view! {
        <div class="publication-compact-banner">
            <div class="compact-banner-left">
                <span class="banner-badge">"PROPOSED DISPATCH"</span>
                <span class="banner-text">"241 bytes to " <strong>"/rosint"</strong> " · 3 tags"</span>
            </div>
            <div class="compact-banner-center">
                <div class="banner-fee-pill">
                    <span class="fee-sats">"241 sats"</span>
                    <span class="fee-rate">"(1 sat/vB)"</span>
                </div>
            </div>
            <div class="compact-banner-right">
                <button type="button" class="btn btn-primary btn-sm">
                    "Approve & Sign ↗"
                </button>
                <button type="button" class="btn btn-secondary btn-sm">
                    "Discard"
                </button>
            </div>
        </div>
    }
}
