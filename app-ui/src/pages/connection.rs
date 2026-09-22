use crate::components::DescribedGroup;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::{component, view};
#[cfg(target_arch = "wasm32")]
use tracing::warn;
use wire_app_shared::connection::ConnectionHealth;

#[component]
pub fn ConnectionPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.check_connection();
    let checking = move || state.is_checking_connection.get();

    view! {
        <section class="page-container connection-page">
            <PageHeader
                eyebrow="CHAIN STATUS."
                title="Your"
                accent="connection."
                subtitle="A read-only check of the selected chain source and your locally indexed history."
            />
            <div class="connection-actions">
                <button type="button" class="btn btn-primary" disabled=checking
                    on:click=move |_clicked| state.check_connection()>
                    {move || match checking() { true => "Checking...", false => "Refresh connection" }}
                </button>
                <span role="status">{move || match checking() { true => "Checking source height...", false => "" }}</span>
            </div>
            {move || match state.connection_health.get() {
                Some(health) => view! { <Health health=health /> }.into_any(),
                None => view! {
                    <DescribedGroup title="Never checked" description="Refresh to check the chain source." >
                        <span />
                    </DescribedGroup>
                }.into_any(),
            }}
            <p class="connection-hint">"Checking does not index new blocks. Open the " <a href="/">"Feed"</a> " to refresh indexed posts."</p>
        </section>
    }
}

#[component]
fn Health(health: ConnectionHealth) -> impl IntoView {
    let state = expect_context::<AppState>();
    let trouble = Trouble::read(&health);
    let remote = height_label(health.remote_height, "Unknown", trouble.stale());
    let local = height_label(health.local_height, "No blocks indexed", false);
    let range = indexed_range(health.local_start, health.local_height);
    let checked = time_label(health.checked_at_ms);
    view! {
        <SettingsGroup title="Chain source">
            <p class="group-title">{health.status()}</p>
            <p class="group-desc">{trouble.explained()}</p>
            {match health.error {
                Some(error) => view! { <p class="alert alert-info" role="alert">{error}</p> }.into_any(),
                None => ().into_any(),
            }}
            <dl class="connection-details">
                <div><dt>"Network"</dt><dd>{health.network}</dd></div>
                <div><dt>"Source"</dt><dd>{health.source_type}</dd></div>
                <div><dt>"Endpoint"</dt><dd>{health.endpoint}</dd></div>
                <div><dt>"Remote block height"</dt><dd>{remote}</dd></div>
                <div><dt>"Locally indexed height"</dt><dd>{local}</dd></div>
                <div><dt>"Indexed range"</dt><dd>{range}</dd></div>
                <div><dt>"Last check"</dt><dd>{checked}</dd></div>
                <div><dt>"Last successful check"</dt><dd>{move || match state.connection_last_success_ms.get() {
                    Some(ms) => time_label(ms),
                    None => "Never".to_owned(),
                }}</dd></div>
            </dl>
        </SettingsGroup>
    }
}

enum Trouble {
    SourceUnverified,
    IndexUnreadable,
    Healthy,
}

impl Trouble {
    fn read(health: &ConnectionHealth) -> Self {
        let Some(error) = health.error.as_deref() else {
            return Trouble::Healthy;
        };
        match error == "Local index unavailable" {
            true => Trouble::IndexUnreadable,
            false => Trouble::SourceUnverified,
        }
    }

    fn stale(&self) -> bool {
        matches!(self, Trouble::SourceUnverified)
    }

    fn explained(&self) -> &'static str {
        match self {
            Trouble::SourceUnverified => {
                "The source could not be verified at this check. Any earlier height is stale."
            }
            Trouble::IndexUnreadable => {
                "Source height was checked, but the local index could not be read."
            }
            Trouble::Healthy => "Source height was checked now. Index height is a local snapshot.",
        }
    }
}

fn height_label(height: Option<u64>, absent: &str, stale: bool) -> String {
    let Some(height) = height else {
        return absent.to_owned();
    };
    match stale {
        true => format!("#{height} (stale)"),
        false => format!("#{height}"),
    }
}

fn indexed_range(start: Option<u64>, end: Option<u64>) -> String {
    let Some(start) = start else {
        return "None".to_owned();
    };
    let Some(end) = end else {
        return "None".to_owned();
    };
    format!("#{start}–#{end}")
}

#[cfg(target_arch = "wasm32")]
fn time_label(ms: u64) -> String {
    if ms == 0 {
        return "Never".to_owned();
    }
    match u32::try_from(ms / 1000) {
        Ok(seconds) => connection_time(f64::from(seconds) * 1000.0),
        Err(error) => {
            warn!(error = %error, "check timestamp is beyond the printable range");
            "Unknown".to_owned()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn time_label(ms: u64) -> String {
    if ms == 0 {
        return "Never".to_owned();
    }
    ms.to_string()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function connection_time(ms) { return new Date(ms).toLocaleString(); }"
)]
extern "C" {
    fn connection_time(ms: f64) -> String;
}
