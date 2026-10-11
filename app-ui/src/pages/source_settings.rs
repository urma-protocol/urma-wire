use crate::components::Actions;
use crate::components::DescribedGroup;
use crate::components::Notice;
use crate::data::chain;
use crate::platform::use_platform;
use crate::platform::Platform;
use crate::state::AppState;
use crate::state::Said;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, event_target_value, view};
use tracing::warn;
use wire_app_core::connection::{SourceChoice, SourceStatus};

#[derive(Clone, Copy)]
struct SourceForm {
    status: RwSignal<Option<SourceStatus>>,
    picked: RwSignal<SourceChoice>,
    url: RwSignal<String>,
    said: RwSignal<Said>,
    busy: RwSignal<bool>,
    state: AppState,
}

impl SourceForm {
    fn adopted(self, status: SourceStatus) {
        match &status.choice {
            SourceChoice::Esplora(url) => self.url.set(url.clone()),
            SourceChoice::Public | SourceChoice::LocalNode => (),
        }
        self.picked.set(status.choice.clone());
        self.status.set(Some(status));
    }

    fn load(self) {
        leptos::task::spawn_local(async move {
            match chain::source().await {
                Ok(status) => self.adopted(status),
                Err(fault) => {
                    warn!(fault = %fault, "configured chain source could not be read");
                    self.said.set(Said::Something(fault.message));
                }
            }
        });
    }

    fn choice(self, browser: bool) -> SourceChoice {
        match browser {
            true => SourceChoice::Esplora(self.url.get_untracked()),
            false => self.picked.get_untracked(),
        }
    }

    fn adopt(self, browser: bool) {
        self.busy.set(true);
        self.said.set(Said::Nothing);
        let choice = self.choice(browser);
        leptos::task::spawn_local(async move {
            match chain::adopt_source(choice).await {
                Ok(status) => {
                    self.adopted(status);
                    self.state.source_changed();
                    self.said.set(Said::Something(
                        "Source verified against the Litecoin Testnet genesis and saved."
                            .to_owned(),
                    ));
                }
                Err(fault) => {
                    warn!(fault = %fault, "chain source refused");
                    self.said.set(Said::Something(fault.message));
                }
            }
            self.busy.set(false);
        });
    }
}

#[component]
pub fn ChainSourceSettings() -> impl IntoView {
    let browser = use_platform() == Platform::Browser;
    let form = SourceForm {
        status: RwSignal::new(None),
        picked: RwSignal::new(SourceChoice::Public),
        url: RwSignal::new(String::new()),
        said: RwSignal::new(Said::Nothing),
        busy: RwSignal::new(false),
        state: expect_context::<AppState>(),
    };
    form.load();
    view! {
        <DescribedGroup
            title="Chain source"
            description="Wire reads blocks and submits your exact signed transactions through this source. No keys, passwords or unlock material are ever sent to any source; only public chain queries and signed bytes leave this device."
        >
            {move || match browser {
                true => view! { <EsploraField form=form /> }.into_any(),
                false => view! { <NativeChoices form=form /> }.into_any(),
            }}
            <Actions>
                <button class="btn btn-primary btn-sm" disabled=move || form.busy.get()
                    on:click=move |_clicked| form.adopt(browser)>
                    {move || match form.busy.get() { true => "Verifying…", false => "Verify & use" }}
                </button>
            </Actions>
            <Notice message=form.said.into() />
        </DescribedGroup>
    }
}

#[component]
fn EsploraField(form: SourceForm) -> impl IntoView {
    view! {
        <label class="field">"Litecoin Testnet Esplora API URL"
            <input type="url" autocomplete="off" prop:value=move || form.url.get()
                on:input=move |event| form.url.set(event_target_value(&event)) />
            <span class="field-hint">"Must allow browser CORS requests. Examples: https://litecoinspace.org/testnet/api or https://testnetscan.com/ltc-testnet/api"</span>
        </label>
    }
}

#[component]
fn NativeChoices(form: SourceForm) -> impl IntoView {
    let pick = move |choice: SourceChoice| move |_changed| form.picked.set(choice.clone());
    let local_ready = move || {
        let Some(status) = form.status.get() else {
            return false;
        };
        status.local_node_configured
    };
    view! {
        <div class="radio-options">
            <label class="radio-label">
                <input type="radio" name="chain-source"
                    prop:checked=move || form.picked.get() == SourceChoice::Public
                    on:change=pick(SourceChoice::Public) />
                <div class="radio-content">
                    <strong>"Public sources"</strong>
                    <span>{move || public_hosts(form.status.get())}</span>
                </div>
            </label>
            <label class="radio-label">
                <input type="radio" name="chain-source" disabled=move || !local_ready()
                    prop:checked=move || form.picked.get() == SourceChoice::LocalNode
                    on:change=pick(SourceChoice::LocalNode) />
                <div class="radio-content">
                    <strong>"Local node"</strong>
                    <span>{move || local_hint(form.status.get())}</span>
                </div>
            </label>
        </div>
    }
}

fn public_hosts(status: Option<SourceStatus>) -> String {
    let Some(status) = status else {
        return "Reading the configured source…".to_owned();
    };
    match status.choice {
        SourceChoice::Public => format!(
            "Litecoin Testnet providers: {}. A configured local node is detected automatically and preferred for block reads when ready. Evidence tiers: light-client inclusion beats public provider observation, and only a local validating node checks blocks itself.",
            status.endpoints.join(", ")
        ),
        SourceChoice::LocalNode | SourceChoice::Esplora(_) => {
            "Litecoin Testnet providers from urma-runtime, verified by genesis when selected. Works with the local node off; automatically uses a configured local node for block reads when ready. Evidence tiers: light-client inclusion beats public provider observation, and only a local validating node checks blocks itself.".to_owned()
        }
    }
}

fn local_hint(status: Option<SourceStatus>) -> String {
    let Some(status) = status else {
        return "Reading the configured source…".to_owned();
    };
    let Some(endpoint) = status.local_endpoint else {
        return "Not configured: set rpc_url and node_auth_file in the URMA config.json to enable it.".to_owned();
    };
    match (status.local_node_configured, status.cookie_available) {
        (true, true) => format!("Cookie-authenticated RPC at {endpoint}; genesis checked before use."),
        (true, false) => format!("{endpoint} is configured but its cookie file is missing. Start litecoind first or stay on Public sources."),
        (false, _cookie) => "Not configured: set rpc_url and node_auth_file in the URMA config.json to enable it.".to_owned(),
    }
}
