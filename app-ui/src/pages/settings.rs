use crate::components::Actions;
use crate::components::DescribedGroup;
use crate::components::Fact;
use crate::components::Facts;
use crate::components::FollowedTopics;
use crate::components::Notice;
use crate::components::PageHeader;
use crate::components::SettingsGroup;
use crate::data::preferences::remember_autoload;
use crate::data::preferences::remember_replies_shown;
use crate::pages::source_settings::ChainSourceSettings;
use crate::platform::use_platform;
use crate::state::AppState;
use crate::state::Said;
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
use leptos::prelude::{component, event_target, event_target_value, view};
use tracing::warn;
use wire_app_core::model::Chain;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <section class="page-container settings-page">
            <PageHeader eyebrow="PREFERENCES." title="Your" accent="settings.">
                "Network, media privacy and runtime preferences. Funding and balances live in Wallet."
            </PageHeader>
            <Notice message=state.status_message.into() />
            <NetworkChoice />
            <DescribedGroup
                title="Followed topics"
                description="Wires and hashtags you follow are suggested first while writing, and can filter the feed."
            >
                <FollowedTopics />
            </DescribedGroup>
            <ReplyPreview />
            <MediaPrivacy />
            <ChainSourceSettings />
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
                        <span>"Reads and publishes through the chain source selected below."</span>
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
fn ReplyPreview() -> impl IntoView {
    let state = expect_context::<AppState>();
    let change = move |event: leptos::ev::Event| {
        let typed = event_target_value(&event);
        let count = match typed.trim().parse::<usize>() {
            Ok(count) => count,
            Err(error) => {
                warn!(error = %error, typed = %typed, "reply preview size is not a whole number");
                event_target::<web_sys::HtmlInputElement>(&event)
                    .set_value(&state.replies_shown.get_untracked().to_string());
                return;
            }
        };
        match remember_replies_shown(count) {
            Ok(()) => state.replies_shown.set(count),
            Err(fault) => {
                warn!(fault = %fault, "reply preview size could not be stored");
                state.status_message.set(Said::Something(fault.message));
            }
        }
    };
    view! {
        <DescribedGroup
            title="Replies in the feed"
            description="The newest replies appear under each post; the full thread opens from the post. 0 keeps replies behind the reply count."
        >
            <label class="field field-count">
                <span>"Replies shown per post"</span>
                <input type="number" min="0" step="1"
                    prop:value=move || state.replies_shown.get().to_string()
                    on:change=change />
            </label>
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
            Err(fault) => {
                warn!(fault = %fault, "image preference could not be stored");
                state.status_message.set(Said::Something(fault.message));
            }
        }
    };
    let clear = move |_clicked: leptos::ev::MouseEvent| {
        state.media_epoch.update(|epoch| *epoch += 1);
        state.media_bytes.set(0);
    };
    view! {
        <DescribedGroup
            title="External images"
            description="Markdown image references are fetched from their hosts; external media is never stored on-chain (URMA Wire D-020)."
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
            <Actions>
                <button class="btn btn-secondary btn-sm" on:click=clear>"Clear displayed media"</button>
            </Actions>
        </DescribedGroup>
    }
}

#[component]
fn RuntimeFacts() -> impl IntoView {
    let platform = use_platform();
    let platform_name = platform.name();
    let storage_name = platform.storage_name();
    view! {
        <SettingsGroup title="Runtime">
            <Facts>
                <Fact label="Platform">{platform_name}</Fact>
                <Fact label="Protocol">"URMA V0"</Fact>
                <Fact label="Local index" wide=true>{storage_name}</Fact>
            </Facts>
        </SettingsGroup>
    }
}
