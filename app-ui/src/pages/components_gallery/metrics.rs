use leptos::prelude::*;

/// Variant A: Network & Chain Status Tile
/// Visualizes current blockchain sync health, tip height, and Esplora endpoint responsiveness.
#[component]
pub fn NetworkMetricCard() -> impl IntoView {
    view! {
        <div class="metric-card-modern">
            <div class="metric-card-header">
                <div class="metric-icon-title">
                    <span class="metric-dot live"></span>
                    <span class="metric-category">"CONSENSUS STATUS"</span>
                </div>
                <span class="badge badge-conf">"LTC TESTNET"</span>
            </div>

            <div class="metric-main-val">
                <span class="metric-number">"4,892,720"</span>
                <span class="metric-unit">"Current Tip Height"</span>
            </div>

            <div class="metric-sub-bar">
                <div class="metric-sub-item">
                    <span class="sub-lbl">"Recent Sync:"</span>
                    <span class="sub-val">"100 / 100 Blocks Indexed"</span>
                </div>
                <div class="metric-sub-item">
                    <span class="sub-lbl">"Latency:"</span>
                    <span class="sub-val">"124 ms"</span>
                </div>
            </div>
        </div>
    }
}

/// Variant B: Spendable Funding Gauge
/// Detailed satoshi breakdown showing spendable UTXOs and publishing readiness.
#[component]
pub fn SpendableGaugeWidget() -> impl IntoView {
    view! {
        <div class="metric-card-modern">
            <div class="metric-card-header">
                <div class="metric-icon-title">
                    <span class="metric-dot lock-active"></span>
                    <span class="metric-category">"SPENDABLE FUNDING"</span>
                </div>
                <span class="badge badge-block">"1 UTXO Ready"</span>
            </div>

            <div class="metric-main-val">
                <span class="metric-number">"100,000"</span>
                <span class="metric-unit">"litoshi spendable"</span>
            </div>

            <div class="metric-sub-bar">
                <div class="metric-sub-item">
                    <span class="sub-lbl">"Est. Capacity:"</span>
                    <span class="sub-val">"~414 Wire Dispatches"</span>
                </div>
                <div class="metric-sub-item">
                    <span class="sub-lbl">"Key Status:"</span>
                    <span class="sub-val">"Unlocked (Session)"</span>
                </div>
            </div>
        </div>
    }
}
