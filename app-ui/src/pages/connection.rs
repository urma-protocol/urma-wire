use crate::components::Fact;
use crate::components::Facts;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::components::Status;
use crate::components::Tone;
use crate::config::{NEVER_CHECKED, NO_INDEXED_RANGE, UNKNOWN_HEIGHT};
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
use wire_app_core::connection::{ConnectionHealth, Trouble};

#[component]
pub fn ConnectionPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    state.check_connection();
    let checking = move || state.is_checking_connection.get();

    view! {
        <section class="page-container connection-page">
            <PageHeader eyebrow="CHAIN STATUS." title="Your" accent="connection.">
                "A read-only check of the selected chain source and your locally indexed history."
            </PageHeader>
            <div class="connection-actions">
                <button type="button" class="btn btn-primary" disabled=checking
                    on:click=move |_clicked| state.check_connection()>
                    {move || match checking() { true => "Checking…", false => "Refresh connection" }}
                </button>
                <span class="sync-status" role="status">{move || match checking() { true => "Checking source height…", false => "" }}</span>
            </div>
            {move || match state.connection_health.get() {
                Some(health) => view! { <Health health=health /> }.into_any(),
                None => view! {
                    <SettingsGroup title="Chain source">
                        <p class="group-desc">"Not checked yet. Refresh to query the source."</p>
                    </SettingsGroup>
                }.into_any(),
            }}
            <p class="connection-hint">"Checking does not index new blocks. Open the " <a href="/">"Feed"</a> " to refresh indexed posts."</p>
        </section>
    }
}

#[component]
fn Health(health: ConnectionHealth) -> impl IntoView {
    let state = expect_context::<AppState>();
    let stale = matches!(health.trouble, Trouble::SourceUnverified(_));
    let remote = height_label(health.remote_height, UNKNOWN_HEIGHT, stale);
    let local = height_label(health.local_height, "No blocks indexed", false);
    let range = indexed_range(health.local_start, health.local_height);
    let checked = time_label(health.checked_at_ms);
    view! {
        <SettingsGroup title="Chain source">
            <div class="state-line">
                <Status tone=tone(&health.trouble) text=health.status() />
                <p class="group-desc">{explained(&health.trouble)}</p>
            </div>
            {match &health.trouble {
                Trouble::Clear => ().into_any(),
                trouble => view! { <p class="alert alert-info" role="alert">{trouble.message().to_owned()}</p> }.into_any(),
            }}
            <Facts>
                <Fact label="Network">{health.network}</Fact>
                <Fact label="Source">{health.source_type}</Fact>
                <Fact label="Endpoint" wide=true>{health.endpoint}</Fact>
                <Fact label="Remote block height">{remote}</Fact>
                <Fact label="Locally indexed height">{local}</Fact>
                <Fact label="Indexed range">{range}</Fact>
                <Fact label="Last check">{checked}</Fact>
                <Fact label="Last successful check">{move || last_success(state)}</Fact>
            </Facts>
        </SettingsGroup>
    }
}

fn tone(trouble: &Trouble) -> Tone {
    match trouble {
        Trouble::Clear => Tone::Live,
        Trouble::IndexUnreadable(_message) => Tone::Trouble,
        Trouble::SourceUnverified(_message) => Tone::Trouble,
    }
}

fn explained(trouble: &Trouble) -> &'static str {
    match trouble {
        Trouble::SourceUnverified(_message) => {
            "The source could not be verified at this check. Any earlier height is stale."
        }
        Trouble::IndexUnreadable(_message) => {
            "Source height was checked, but the local index could not be read."
        }
        Trouble::Clear => "Source height was checked now. Index height is a local snapshot.",
    }
}

fn last_success(state: AppState) -> String {
    let Some(ms) = state.connection_last_success_ms.get() else {
        return NEVER_CHECKED.to_owned();
    };
    time_label(ms)
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
        return NO_INDEXED_RANGE.to_owned();
    };
    let Some(end) = end else {
        return NO_INDEXED_RANGE.to_owned();
    };
    format!("#{start}–#{end}")
}

#[cfg(target_arch = "wasm32")]
fn time_label(ms: u64) -> String {
    if ms == 0 {
        return NEVER_CHECKED.to_owned();
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
        return NEVER_CHECKED.to_owned();
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
