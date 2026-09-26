use crate::components::Actions;
use crate::components::DescribedGroup;
use crate::components::Notice;
use crate::data::chain;
use crate::platform::use_platform;
use crate::platform::Platform;
use crate::state::Said;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Show;
use leptos::prelude::{component, event_target_value, view};
use tracing::warn;

#[component]
pub fn BrowserSourceSettings() -> impl IntoView {
    let browser = use_platform() == Platform::Browser;
    let source = RwSignal::new(String::new());
    let status = RwSignal::new(Said::Nothing);
    let busy = RwSignal::new(false);
    if browser {
        leptos::task::spawn_local(async move {
            match chain::source().await {
                Ok(url) => source.set(url),
                Err(fault) => {
                    warn!(fault = %fault, "configured chain source could not be read");
                    status.set(Said::Something(fault.message));
                }
            }
        });
    }
    let save = move |_clicked: leptos::ev::MouseEvent| {
        busy.set(true);
        let url = source.get_untracked();
        leptos::task::spawn_local(async move {
            match chain::adopt_source(url).await {
                Ok(()) => status.set(Said::Something(
                    "Source saved after Litecoin Testnet genesis verification.".to_owned(),
                )),
                Err(fault) => {
                    warn!(fault = %fault, "chain source refused");
                    status.set(Said::Something(fault.message));
                }
            }
            busy.set(false);
        });
    };
    view! {
        <Show when=move || browser>
            <DescribedGroup
                title="Public chain source"
                description="Wire contacts this source directly from your browser. It must allow browser CORS requests. No keys or passwords are sent. Offline history remains available; sync and publication need a reachable source."
            >
                <label class="field">"Litecoin Testnet Esplora API URL"
                    <input type="url" autocomplete="off" prop:value=move || source.get()
                        on:input=move |event| source.set(event_target_value(&event)) />
                    <span class="field-hint">"Examples: https://litecoinspace.org/testnet/api or https://testnetscan.com/ltc-testnet/api"</span>
                </label>
                <Actions>
                    <button class="btn btn-primary btn-sm" disabled=move || busy.get() on:click=save>
                        {move || match busy.get() { true => "Verifying…", false => "Verify & save source" }}
                    </button>
                </Actions>
                <Notice message=status.into() />
            </DescribedGroup>
        </Show>
    }
}
