use crate::components::Said;
use crate::components::Warning;
use crate::data::publication;
use crate::data::publication::Addressed;
use crate::data::publication::Draft;
use crate::limits::AVATAR_PREVIEW_PIXELS;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::InnerHtmlAttribute;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::Read;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, event_target_value, view};
use tracing::warn;
use wire_app_shared::live_model::Publication;
use wire_app_shared::render_avatar_svg;

#[component]
pub fn ComposeBox(#[prop(default = Addressed::Standalone)] addressed: Addressed) -> impl IntoView {
    let state = expect_context::<AppState>();
    let text = match addressed {
        Addressed::Thread(_) => RwSignal::new(String::new()),
        Addressed::Standalone => state.post_draft,
    };
    let wire = RwSignal::new(String::new());
    let hashtags = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let trouble = RwSignal::new(Said::Nothing);
    let max_bytes = addressed.max_bytes();
    let prepare = move |_clicked: leptos::ev::MouseEvent| {
        busy.set(true);
        trouble.set(Said::Nothing);
        let draft = Draft::spoken(text.get(), &wire.get(), &hashtags.get(), addressed);
        let chain = state.active_chain.get();
        leptos::task::spawn_local(async move {
            match publication::quote(chain, draft).await {
                Ok(quote) => state.publication.set(Some(quote)),
                Err(failure) => {
                    warn!(failure = %failure, "publication quote refused");
                    trouble.set(Said::Something(failure.reported()));
                }
            }
            busy.set(false);
        });
    };
    let blocked = move || {
        busy.get()
            || state.following.get()
            || text.read().trim().is_empty()
            || text.read().len() > max_bytes
    };
    view! {
        <div class="compose-card compact-composer">
            <textarea class="compose-textarea" rows="2" aria-label="Post text" placeholder="What’s on your mind?"
                prop:value=move || text.get() on:input=move |ev| text.set(event_target_value(&ev)) />
            <details class="compose-topics">
                <summary>"Wire & hashtags"</summary>
                <div class="topic-fields">
                <label>"Wire (optional)"<input class="input" placeholder="/rosint" prop:value=move || wire.get() on:input=move |ev| wire.set(event_target_value(&ev)) /></label>
                <label>"Hashtags (optional)"<input class="input" placeholder="#investigation #documents" prop:value=move || hashtags.get() on:input=move |ev| hashtags.set(event_target_value(&ev)) /></label>
                </div>
            </details>
            <div class="compose-footer">
                <span class="byte-meter">{move || text.read().len()} "/" {max_bytes} " bytes"</span>
                <button class="btn btn-primary btn-sm" on:click=prepare disabled=blocked>
                    {move || match busy.get() { true => "Preparing exact quote…", false => "Review & publish" }}
                </button>
            </div>
            <Warning message=trouble.into() />
        </div>
        <PublicationPanel />
    }
}

#[component]
pub fn PublicationPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || match awaiting(state) {
            Standing::Awaiting(publication) => view! { <PublicationCard publication=publication /> }.into_any(),
            Standing::Nothing => ().into_any(),
        }}
        <Warning message=state.publication_error.into() />
    }
}

enum Standing {
    Awaiting(Publication),
    Nothing,
}

fn awaiting(state: AppState) -> Standing {
    let Some(publication) = state.publication.get() else {
        return Standing::Nothing;
    };
    match publication.complete {
        true => Standing::Nothing,
        false => Standing::Awaiting(publication),
    }
}

#[component]
fn PublicationCard(publication: Publication) -> impl IntoView {
    let state = expect_context::<AppState>();
    let id = publication.id.clone();
    let approve = !publication.approved;
    let kind = match publication.kind.as_str() {
        "profile" => "Nickname update · kind 05",
        "avatar" => "Avatar update · kind 06",
        _ => "Post / reply",
    };
    let topics = topic_labels(&publication);
    let avatar = frozen_avatar(&publication);
    view! {
        <section class="compose-card publication-card">
            <div class="compose-header"><strong>"Your publication"</strong><span class="chain-badge-small">"LTC TESTNET"</span></div>
            <p><strong>{kind}</strong></p>
            <p class="publication-text">{publication.text.clone()}</p>
            {match avatar {
                Frozen::Avatar(svg) => view! {
                    <div inner_html=svg />
                    <p>"Frozen avatar preview · 512 pixel bytes / 520 bytes total"</p>
                }.into_any(),
                Frozen::Malformed(reason) => view! { <p class="alert alert-error" role="alert">{reason}</p> }.into_any(),
                Frozen::None => ().into_any(),
            }}
            {match topics {
                Some(labels) => view! { <p class="topic-labels">{labels}</p> }.into_any(),
                None => ().into_any(),
            }}
            {match publication.target.clone() {
                Some(target) => view! { <p>"Reply to: " <code>{target}</code></p> }.into_any(),
                None => ().into_any(),
            }}
            <div class="publication-costs">
                <div><span>"Network fee"</span><strong class="accent-text">{publication.fee}<small>" litoshi"</small></strong></div>
                <div><span>"Selected funding"</span><strong>{publication.selected_funding}<small>" litoshi"</small></strong></div>
                <div><span>"Returned to you"</span><strong>{publication.returned}<small>" litoshi"</small></strong></div>
            </div>
            <div class="publication-status" role="status"><span>{publication.confirmed} "/2 confirmed"</span><span>{publication.mempool} " in mempool"</span></div>
            <progress class="publication-progress" max="2" value=publication.confirmed aria-label="Confirmed transactions"></progress>
            <p class="action-desc">{publication.message.clone()}</p>
            <details class="transaction-details"><summary>"Transaction details"</summary><code>{publication.txid.clone()}</code></details>
            <p class="action-desc">"Approval publishes the exact record shown above and spends this fee. Editing requires a new review."</p>
            <div class="publication-actions">
                <button class="btn btn-primary" disabled=move || state.following.get()
                    on:click=move |_| state.follow(id.clone(), approve)>
                    {match approve { true => "Approve fee & publish", false => "Resume publication" }}
                </button>
                <button class="btn btn-secondary" disabled=move || !state.following.get()
                    on:click=move |_| state.pause_requested.set(true)>"Pause after current request"</button>
            </div>
            <p class="action-desc">"You can close Wire and resume later with the same plan and fee."</p>
        </section>
    }
}

fn topic_labels(publication: &Publication) -> Option<String> {
    if publication.kind == "profile" || publication.kind == "avatar" {
        return None;
    }
    let wire = match publication.wire.is_empty() {
        true => "No wire".to_owned(),
        false => format!("/{}", publication.wire),
    };
    let tags: Vec<String> = publication
        .hashtags
        .iter()
        .map(|tag| format!("#{tag}"))
        .collect();
    Some(format!("{wire} {}", tags.join(" ")))
}

enum Frozen {
    Avatar(String),
    Malformed(String),
    None,
}

fn frozen_avatar(publication: &Publication) -> Frozen {
    let Some(bytes) = publication.avatar.as_ref() else {
        return Frozen::None;
    };
    match <&[u8; 512]>::try_from(bytes.as_slice()) {
        Ok(pixels) => Frozen::Avatar(render_avatar_svg(pixels, AVATAR_PREVIEW_PIXELS)),
        Err(error) => {
            warn!(error = %error, "quoted avatar is not 512 pixel bytes");
            Frozen::Malformed(format!(
                "Quoted avatar carries {} bytes, not 512; it cannot be previewed.",
                bytes.len()
            ))
        }
    }
}
