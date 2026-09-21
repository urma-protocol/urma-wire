use crate::state::AppState;
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function connection_time(ms) { return new Date(ms).toLocaleString(); }"
)]
extern "C" {
    fn connection_time(ms: f64) -> String;
}

fn time_label(ms: u64) -> String {
    if ms == 0 {
        return "Never".into();
    }
    #[cfg(target_arch = "wasm32")]
    {
        connection_time(ms as f64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ms.to_string()
    }
}

#[component]
pub fn ConnectionPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.check_connection();

    view! {
        <section class="page-container connection-page">
            <div><span class="eyebrow">"CHAIN STATUS."</span><h1 class="page-title">"Your " <span class="accent-text">"connection."</span></h1></div>
            <p class="page-subtitle">"A read-only check of the selected chain source and your locally indexed history."</p>
            <div class="connection-actions">
                <button type="button" class="btn btn-primary" disabled=move || state.is_checking_connection.get() on:click=move |_| state.check_connection()>
                    {move || if state.is_checking_connection.get() { "Checking..." } else { "Refresh connection" }}
                </button>
                <span role="status">{move || if state.is_checking_connection.get() { "Checking source height..." } else { "" }}</span>
            </div>
            {move || {
                let snapshot = state.connection_health.get();
                let last_success = state.connection_last_success_ms.get();
                match snapshot {
                    Some(health) => {
                        let failed = health.error.is_some();
                        let source_failed = failed && health.error.as_deref() != Some("Local index unavailable");
                        let remote = health.remote_height.map(|height| format!("#{height}")).unwrap_or_else(|| "Unknown".into());
                        let local = health.local_height.map(|height| format!("#{height}")).unwrap_or_else(|| "No blocks indexed".into());
                        let range = match (health.local_start, health.local_height) {
                            (Some(start), Some(end)) => format!("#{start}–#{end}"),
                            _ => "None".into(),
                        };
                        view! {
                            <div class="settings-group connection-summary">
                                <h3 class="group-title">{health.status()}</h3>
                                <p class="group-desc">{if source_failed { "The source could not be verified at this check. Any earlier height is stale." } else if failed { "Source height was checked, but the local index could not be read." } else { "Source height was checked now. Index height is a local snapshot." }}</p>
                                {health.error.map(|error| view! { <p class="alert alert-info" role="alert">{error}</p> })}
                                <dl class="connection-details">
                                    <div><dt>"Network"</dt><dd>{health.network}</dd></div>
                                    <div><dt>"Source"</dt><dd>{health.source_type}</dd></div>
                                    <div><dt>"Endpoint"</dt><dd>{health.endpoint}</dd></div>
                                    <div><dt>"Remote block height"</dt><dd>{if source_failed && health.remote_height.is_some() { format!("{remote} (stale)") } else { remote }}</dd></div>
                                    <div><dt>"Locally indexed height"</dt><dd>{local}</dd></div>
                                    <div><dt>"Indexed range"</dt><dd>{range}</dd></div>
                                    <div><dt>"Last check"</dt><dd>{time_label(health.checked_at_ms)}</dd></div>
                                    <div><dt>"Last successful check"</dt><dd>{last_success.map(time_label).unwrap_or_else(|| "Never".into())}</dd></div>
                                </dl>
                            </div>
                        }.into_any()
                    }
                    None => view! { <div class="settings-group"><h3 class="group-title">"Never checked"</h3><p>"Refresh to check the chain source."</p></div> }.into_any(),
                }
            }}
            <p class="connection-hint">"Checking does not index new blocks. To continue catch-up, open the " <a href="/">"Feed"</a> " and refresh it."</p>
        </section>
    }
}
