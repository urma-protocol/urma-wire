use crate::components::Said;
use crate::components::SettingsGroup;
use crate::data::chain;
use crate::platform::use_platform_bridge;
use crate::platform::PlatformType;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GlobalAttributes;
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
    let browser = use_platform_bridge().platform_type() == PlatformType::Web;
    let source = RwSignal::new(String::new());
    let status = RwSignal::new(Said::Nothing);
    let busy = RwSignal::new(false);
    if browser {
        leptos::task::spawn_local(async move {
            match chain::source().await {
                Ok(url) => source.set(url),
                Err(failure) => {
                    warn!(failure = %failure, "configured chain source could not be read");
                    status.set(Said::Something(failure.reported()));
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
                Err(failure) => {
                    warn!(failure = %failure, "chain source refused");
                    status.set(Said::Something(failure.reported()));
                }
            }
            busy.set(false);
        });
    };
    view! {
        <Show when=move || browser>
            <SettingsGroup title="Public chain source">
                <p>"Wire contacts this source directly from your browser. It must allow browser CORS requests. No keys or passwords are sent. Offline history remains available; sync and publication need a reachable source."</p>
                <label>"Litecoin Testnet Esplora API URL"
                    <input type="url" prop:value=move || source.get()
                        on:input=move |event| source.set(event_target_value(&event)) />
                </label>
                <button disabled=move || busy.get() on:click=save>"Verify & save source"</button>
                <p>"Examples: https://litecoinspace.org/testnet/api or https://testnetscan.com/ltc-testnet/api"</p>
                <p role="status">{move || match status.get() {
                    Said::Something(message) => message,
                    Said::Nothing => String::new(),
                }}</p>
            </SettingsGroup>
        </Show>
    }
}
