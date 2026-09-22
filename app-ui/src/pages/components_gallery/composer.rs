use leptos::prelude::*;

/// Variant A: Baseline V0 Compact Composer
/// The standard collapsed composer with expandable wire/hashtags details and byte count.
#[component]
pub fn ComposerVariantA() -> impl IntoView {
    let text = RwSignal::new("Preliminary intelligence confirms transponder deactivation prior to rendezvous.".to_string());
    let wire = RwSignal::new("rosint".to_string());
    let hashtags = RwSignal::new("#investigation #maritime".to_string());
    let max_bytes = 32_760;

    view! {
        <div class="compose-card compact-composer">
            <textarea
                class="compose-textarea"
                rows="2"
                aria-label="Post text"
                placeholder="What’s on your mind?"
                prop:value=move || text.get()
                on:input=move |ev| text.set(event_target_value(&ev))
            />
            <details class="compose-topics" open>
                <summary>"Wire & hashtags"</summary>
                <div class="topic-fields">
                    <label>"Wire (optional)"
                        <input
                            class="input"
                            placeholder="/rosint"
                            prop:value=move || wire.get()
                            on:input=move |ev| wire.set(event_target_value(&ev))
                        />
                    </label>
                    <label>"Hashtags (optional)"
                        <input
                            class="input"
                            placeholder="#investigation #documents"
                            prop:value=move || hashtags.get()
                            on:input=move |ev| hashtags.set(event_target_value(&ev))
                        />
                    </label>
                </div>
            </details>
            <div class="compose-footer">
                <span class="byte-meter">{move || text.read().len()} "/" {max_bytes} " bytes"</span>
                <button type="button" class="btn btn-primary btn-sm">
                    "Review & publish"
                </button>
            </div>
        </div>
    }
}

/// Variant B: Split Studio / Live Preview Composer
/// Side-by-side markdown editor and live preview pane with live byte gauge and fee estimator.
#[component]
pub fn ComposerVariantB() -> impl IntoView {
    let text = RwSignal::new("## Sensor Correlation Report\n\nSatellite imagery from *Pass 840* aligns with **AIS radar dropout**.\n\n* Coordinates: `44.128° N, 28.654° E`\n* Anomaly: Vessel extinguished transponder 14 nmi prior.\n\nImmutable record verified by BIP-340 key.".to_string());
    let wire = RwSignal::new("rosint".to_string());
    let hashtags = RwSignal::new("satellites ais maritime".to_string());
    let max_bytes = 32_760;
    let byte_count = move || text.read().len();
    let est_fee_sats = move || 154 + (text.read().len() as u64 / 4);

    view! {
        <div class="compose-card composer-studio">
            <div class="studio-header">
                <div class="studio-title-block">
                    <span class="eyebrow">"INTELLIGENCE DISPATCH STUDIO"</span>
                    <h4 class="studio-heading">"Draft Sovereign Publication"</h4>
                </div>
                <div class="studio-metrics">
                    <span class="metric-chip">"Est. Fee: " {est_fee_sats} " sats"</span>
                    <span class="metric-chip">"Rate: 1 sat/vB"</span>
                </div>
            </div>

            <div class="studio-topic-row">
                <div class="channel-selector-inline">
                    <span class="channel-prefix">"/"</span>
                    <input
                        class="channel-input"
                        placeholder="channel (e.g. rosint)"
                        prop:value=move || wire.get()
                        on:input=move |ev| wire.set(event_target_value(&ev))
                    />
                </div>
                <div class="tags-input-inline">
                    <span class="tags-prefix">"#"</span>
                    <input
                        class="tags-input"
                        placeholder="tags separated by space"
                        prop:value=move || hashtags.get()
                        on:input=move |ev| hashtags.set(event_target_value(&ev))
                    />
                </div>
            </div>

            <div class="studio-panes">
                <div class="studio-editor-pane">
                    <div class="pane-label">"Markdown Source"</div>
                    <textarea
                        class="studio-textarea"
                        rows="6"
                        placeholder="Draft dispatch in inert markdown..."
                        prop:value=move || text.get()
                        on:input=move |ev| text.set(event_target_value(&ev))
                    />
                </div>
                <div class="studio-preview-pane">
                    <div class="pane-label">"Live Rendered Output"</div>
                    <div class="studio-preview-content">
                        <div class="preview-channel-line">
                            {move || (!wire.get().is_empty()).then(|| format!("/{}", wire.get()))}
                            " "
                            {move || hashtags.get().split_whitespace().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ")}
                        </div>
                        <div class="preview-text">
                            {move || text.get()}
                        </div>
                    </div>
                </div>
            </div>

            <div class="studio-footer">
                <div class="studio-gauge">
                    <div class="gauge-bar-track">
                        <div
                            class="gauge-bar-fill"
                            style=move || format!("width: {}%;", (byte_count() as f32 / max_bytes as f32 * 100.0).min(100.0))
                        />
                    </div>
                    <span class="byte-meter">{byte_count} "/" {max_bytes} " bytes (exact UTF-8)"</span>
                </div>
                <div class="studio-actions">
                    <button type="button" class="btn btn-secondary btn-sm">"Clear Draft"</button>
                    <button type="button" class="btn btn-primary btn-sm">"Review & Quote Plan →"</button>
                </div>
            </div>
        </div>
    }
}

/// Variant C: Quick Wire Dispatcher
/// High-speed broadcast bar with 1-click channel presets, rapid tag pills, and instant quotation.
#[component]
pub fn ComposerVariantC() -> impl IntoView {
    let text = RwSignal::new(String::new());
    let active_channel = RwSignal::new("rosint".to_string());
    let urgent = RwSignal::new(false);

    let channels = vec!["rosint", "intel", "alerts", "crypto"];

    view! {
        <div class="dispatcher-card">
            <div class="dispatcher-top">
                <span class="dispatcher-badge">"⚡ QUICK DISPATCH BAR"</span>
                <div class="dispatcher-channels">
                    {channels.into_iter().map(|ch| {
                        let is_sel = move || active_channel.get() == ch;
                        view! {
                            <button
                                type="button"
                                class=move || if is_sel() { "ch-pill active" } else { "ch-pill" }
                                on:click=move |_| active_channel.set(ch.to_string())
                            >
                                "/" {ch}
                            </button>
                        }
                    }).collect_view()}
                </div>
                <label class="urgent-toggle">
                    <input
                        type="checkbox"
                        checked=move || urgent.get()
                        on:change=move |ev| urgent.set(event_target_checked(&ev))
                    />
                    <span>"Urgent Advisory"</span>
                </label>
            </div>

            <div class="dispatcher-input-row">
                <input
                    type="text"
                    class="dispatcher-input"
                    placeholder=move || format!("Broadcast brief bulletin to /{}...", active_channel.get())
                    prop:value=move || text.get()
                    on:input=move |ev| text.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    class="btn btn-primary btn-sm dispatcher-btn"
                    disabled=move || text.get().trim().is_empty()
                >
                    "Prepare Quote ↗"
                </button>
            </div>
        </div>
    }
}
