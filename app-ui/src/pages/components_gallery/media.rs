use leptos::prelude::*;

/// Variant A: Security-Inspected External Media Card
/// Visualizes the strict URMA media verification pipeline: public unicast DNS pinning,
/// SHA-256 fingerprinting, MIME enforcement, and zero-cache policy.
#[component]
pub fn MediaVariantA() -> impl IntoView {
    let purged = RwSignal::new(false);

    let purge_cache = move |_| {
        purged.set(true);
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            purged.set(false);
        });
    };

    view! {
        <div class="media-security-card">
            <div class="media-preview-container">
                <div class="mock-satellite-image">
                    <div class="sat-grid-overlay">
                        <span class="sat-coord">"44.128° N, 28.654° E · PASS 840"</span>
                        <span class="sat-tag">"RESOLVED UNICAST IP: 198.51.100.42"</span>
                    </div>
                </div>
            </div>

            <div class="media-audit-tray">
                <div class="audit-badge-row">
                    <span class="sec-badge sec-badge-verified">"✓ DNS Pinned (Public Unicast)"</span>
                    <span class="sec-badge sec-badge-verified">"✓ PNG MIME Validated"</span>
                    <span class="sec-badge sec-badge-verified">"✓ Memory Budget: 1.4 MiB / 64 MiB"</span>
                </div>

                <div class="audit-details-grid">
                    <div class="audit-col">
                        <span class="audit-lbl">"Resource Source URL:"</span>
                        <code class="audit-code">"https://imagery.geoint.org/pass/840_blacksea.png"</code>
                    </div>
                    <div class="audit-col">
                        <span class="audit-lbl">"SHA-256 Digest:"</span>
                        <code class="audit-code">"7f8a92bc4410...331bc08"</code>
                    </div>
                </div>

                <div class="audit-actions">
                    <button type="button" class="btn btn-secondary btn-sm" on:click=purge_cache>
                        {move || if purged.get() { "✓ Media Memory Purged" } else { "Clear From In-Memory Cache" }}
                    </button>
                    <span class="audit-note">"Zero persistent disk/HTTP cache. Loaded into memory only."</span>
                </div>
            </div>
        </div>
    }
}

/// Variant B: Privacy Shield / Quarantined Media Box
/// Displayed when autoload is disabled by the user or when untrusted hosts are referenced.
#[component]
pub fn MediaVariantB() -> impl IntoView {
    let allowed = RwSignal::new(false);

    view! {
        <div class="media-quarantine-box">
            <div class="quarantine-icon-col">
                <span class="quarantine-symbol">"🛡️"</span>
            </div>
            <div class="quarantine-content">
                <div class="quarantine-header">
                    <strong class="quarantine-title">"External Media Quarantined"</strong>
                    <span class="badge badge-warning">"Autoload Blocked"</span>
                </div>
                <p class="quarantine-desc">
                    "This dispatch references an external image from "
                    <code>"imagery.geoint.org"</code>
                    ". Loading it will expose your IP address and network metadata to the host server."
                </p>
                <div class="quarantine-meta-row">
                    <span class="meta-item">"Maximum Size Bound: 4.0 MiB"</span>
                    <span class="meta-item">"Policy: No Cookies · No Referrer"</span>
                </div>
                <div class="quarantine-actions">
                    <button
                        type="button"
                        class="btn btn-primary btn-sm"
                        on:click=move |_| allowed.set(true)
                    >
                        {move || if allowed.get() { "Connecting via Pinned Unicast DNS..." } else { "Fetch & Inspect Image" }}
                    </button>
                    <button type="button" class="btn btn-secondary btn-sm">
                        "Inspect Host Reputation"
                    </button>
                </div>
            </div>
        </div>
    }
}
