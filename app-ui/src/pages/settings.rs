use crate::components::markdown::remember_autoload;
use crate::components::DescribedGroup;
use crate::components::FollowedTopics;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::pages::source_settings::BrowserSourceSettings;
use crate::platform::use_platform_bridge;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::Set;
use leptos::prelude::Update;
use leptos::prelude::{component, view};
use tracing::warn;
use wire_app_shared::Chain;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <section class="page-container settings-page">
            <PageHeader
                eyebrow="PREFERENCES."
                title="Your"
                accent="settings."
                subtitle="Network, media privacy and runtime preferences. Funding and balances live in Wallet."
            />
            <Notice message=state.status_message.into() />
            <NetworkChoice />
            <DescribedGroup
                title="Followed topics"
                description="Wires and hashtags you follow are suggested first while writing, and can filter the feed."
            >
                <FollowedTopics />
            </DescribedGroup>
            <MediaPrivacy />
            <BrowserSourceSettings />
            <RuntimeFacts />
        </section>
    }
}

#[component]
fn NetworkChoice() -> impl IntoView {
    let state = expect_context::<AppState>();
    let choose = move |chain: Chain| move |_changed| state.set_chain(chain);
    view! {
        <DescribedGroup
            title="Network"
            description="This V0 flow uses Litecoin Testnet. The selected node's genesis is checked before operations."
        >
            <div class="radio-options">
                <label class="radio-label">
                    <input type="radio" name="chain"
                        prop:checked=move || state.active_chain.get() == Chain::Litecoin
                        on:change=choose(Chain::Litecoin) />
                    <div class="radio-content">
                        <strong>"Litecoin Testnet"</strong>
                        <span>"Uses the URMA node configuration; public sources when no local RPC is configured."</span>
                    </div>
                </label>
                <label class="radio-label">
                    <input type="radio" name="chain" disabled=true
                        prop:checked=move || state.active_chain.get() == Chain::Bitcoin
                        on:change=choose(Chain::Bitcoin) />
                    <div class="radio-content">
                        <strong>"Bitcoin Testnet4"</strong>
                        <span>"Not connected in this application milestone."</span>
                    </div>
                </label>
            </div>
        </DescribedGroup>
    }
}

#[component]
fn MediaPrivacy() -> impl IntoView {
    let state = expect_context::<AppState>();
    let hide_images = move || !state.autoload_images.get();
    let toggle = move |_changed: leptos::ev::Event| {
        let enabled = !state.autoload_images.get_untracked();
        match remember_autoload(enabled) {
            Ok(()) => state.autoload_images.set(enabled),
            Err(complaint) => {
                warn!("image preference could not be stored");
                state.status_message.set(complaint);
            }
        }
    };
    let clear = move |_clicked: leptos::ev::MouseEvent| {
        state.media_epoch.update(|epoch| *epoch += 1);
        state.media_bytes.set(0);
    };
    view! {
        <DescribedGroup
            title="External Media Fetch (Markdown Images)"
            description="Under the URMA Wire specification (D-020), external media is not stored on-chain."
        >
            <label class="toggle-control">
                <input type="checkbox" prop:checked=hide_images on:change=toggle />
                <span class="toggle-text">"Don't autoload external images"</span>
            </label>
            <div class="privacy-notice">
                <strong>"External media privacy: "</strong>
                "Downloading external images exposes your connection IP address and request metadata to the host server. Disabling autoload prevents automatic metadata leakage."
            </div>
            <p class="action-desc">"Desktop fetches checked raster images. Browser fetching is disabled because destination checks cannot be enforced. Displayed media is temporary; Wire keeps no media cache."</p>
            <button class="btn btn-secondary" on:click=clear>"Clear displayed media"</button>
        </DescribedGroup>
    }
}

#[component]
fn RuntimeFacts() -> impl IntoView {
    let platform = use_platform_bridge();
    view! {
        <SettingsGroup title="Runtime Platform & Local Indexer">
            <div class="runtime-info-table">
                <div class="info-row">
                    <span class="info-label">"Runtime Target:"</span>
                    <span class="info-val">{platform.platform_name()}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"Indexing Engine:"</span>
                    <span class="info-val">{platform.storage_name()}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">"URMA Protocol:"</span>
                    <span class="info-val">"URMA V0"</span>
                </div>
            </div>
        </SettingsGroup>
    }
}
