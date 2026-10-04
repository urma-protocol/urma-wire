use crate::components::Actions;
use crate::components::Warning;
use crate::components::markdown::PostText;
use crate::config::AVATAR_PREVIEW_PIXELS;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::Effect;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GetValue;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::InnerHtmlAttribute;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::NodeRef;
use leptos::prelude::NodeRefAttribute;
use leptos::prelude::OnAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::SetValue;
use leptos::prelude::StoredValue;
use leptos::prelude::{component, view};
use tracing::warn;
use wire_app_core::avatar::render_avatar_svg;
use wire_app_core::markdown::{segments, Segment};
use wire_app_core::publication::{Addressed, Content, Publication};

#[component]
pub fn PublicationPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    let panel: NodeRef<leptos::html::Div> = NodeRef::new();
    let shown = StoredValue::new(String::new());
    Effect::new(move |_ran_before: Option<()>| {
        let Standing::Awaiting(publication) = awaiting(state) else {
            return;
        };
        if shown.get_value() == publication.id {
            return;
        }
        shown.set_value(publication.id);
        let Some(panel) = panel.get_untracked() else {
            return;
        };
        let Some(card) = panel.first_element_child() else {
            return;
        };
        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_block(web_sys::ScrollLogicalPosition::Nearest);
        card.scroll_into_view_with_scroll_into_view_options(&options);
    });
    view! {
        <div node_ref=panel class="publication-panel">
            {move || match awaiting(state) {
                Standing::Awaiting(publication) => view! { <PublicationCard publication=publication /> }.into_any(),
                Standing::Nothing => ().into_any(),
            }}
            <Warning message=state.publication_error.into() />
        </div>
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
    let explained = explained(approve);
    let text = frozen_text(&publication.content);
    view! {
        <section class="compose-card publication-card">
            <div class="compose-header">
                <strong>"Your publication"</strong>
                <span class="chain-badge-small">"LTC TESTNET"</span>
            </div>
            <div class="publication-kind">
                <span class="badge">{kind_label(&publication.content)}</span>
                {topic_labels(&publication.content).into_iter().map(|label| view! { <span class="composer-meta-chip">{label}</span> }).collect_view()}
                {match target(&publication.content) {
                    Target::Reply(target) => view! { <span class="publication-target">"Reply to "<code>{target}</code></span> }.into_any(),
                    Target::Standalone => ().into_any(),
                }}
            </div>
            {match &publication.content {
                Content::Post { wire, hashtags, .. } => view! {
                    <div class="publication-text"><PostText text=text wire=wire.clone() hashtags=hashtags.clone() /></div>
                }.into_any(),
                _ => match text.is_empty() {
                    true => ().into_any(),
                    false => view! { <p class="publication-text">{text}</p> }.into_any(),
                },
            }}
            {match &publication.content {
                Content::Post { text, .. } => {
                    let photos: Vec<_> = segments(text).into_iter().filter_map(|piece| match piece {
                        Segment::Image { url, alt } => Some((url, alt)),
                        Segment::Text(_) => None,
                    }).collect();
                    if photos.is_empty() { ().into_any() } else { view! {
                        <div class="publication-photos">
                            <strong>"External photos in this signed record"</strong>
                            <ul>{photos.into_iter().map(|(url, alt)| view! { <li><span>{alt}</span><code>{url}</code></li> }).collect_view()}</ul>
                        </div>
                    }.into_any() }
                }
                _ => ().into_any(),
            }}
            {match frozen_avatar(&publication.content) {
                Frozen::Avatar(svg) => view! {
                    <div class="avatar-container" inner_html=svg />
                    <p class="action-desc">"Frozen avatar preview · 512 pixel bytes / 520 bytes total"</p>
                }.into_any(),
                Frozen::Malformed(reason) => view! { <p class="alert alert-error" role="alert">{reason}</p> }.into_any(),
                Frozen::Absent => ().into_any(),
            }}
            <div class="publication-costs">
                <div><span>"Network fee"</span><strong class="accent-text">{publication.fee}<small>" litoshi"</small></strong></div>
                <div><span>"Selected funding"</span><strong>{publication.selected_funding}<small>" litoshi"</small></strong></div>
                <div><span>"Returned to you"</span><strong>{publication.returned}<small>" litoshi"</small></strong></div>
            </div>
            <div class="publication-status" role="status">
                <span><strong>{publication.confirmed}"/2"</strong>" confirmed"</span>
                <span>{publication.mempool}" in mempool"</span>
            </div>
            <progress class="publication-progress" max="2" value=publication.confirmed aria-label="Confirmed transactions"></progress>
            <p class="action-desc">{publication.message.clone()}</p>
            <details class="transaction-details"><summary>"Transaction details"</summary><code>{publication.txid.clone()}</code></details>
            <p class="action-desc">{explained}</p>
            <Actions>
                <button class="btn btn-primary" disabled=move || state.following.get()
                    on:click=move |_clicked| state.follow(id.clone(), approve)>
                    {match approve { true => "Approve fee & publish", false => "Resume publication" }}
                </button>
                <button class="btn btn-secondary" disabled=move || !state.following.get()
                    on:click=move |_clicked| state.pause_requested.set(true)>"Pause after current request"</button>
            </Actions>
            <p class="action-desc">"You can close Wire and resume later with the same plan and fee."</p>
            <AbandonPlan id=publication.id.clone() abandonable=publication.abandonable />
        </section>
    }
}

fn explained(approve: bool) -> &'static str {
    match approve {
        true => "Approval publishes exactly the record shown above and spends this fee. Editing requires a new review.",
        false => "Resuming continues the same approved plan: the same signed bytes and the same fee.",
    }
}

#[component]
fn AbandonPlan(id: String, abandonable: bool) -> impl IntoView {
    let state = expect_context::<AppState>();
    let confirming = RwSignal::new(false);
    let abandon_id = id.clone();
    view! {
        {move || match (abandonable, confirming.get()) {
            (false, _either) => ().into_any(),
            (true, false) => view! {
                <Actions>
                    <button class="btn btn-secondary btn-sm" disabled=move || state.following.get()
                        on:click=move |_clicked| confirming.set(true)>"Abandon this plan"</button>
                </Actions>
            }.into_any(),
            (true, true) => {
                let abandon_id = abandon_id.clone();
                view! {
                    <p class="alert alert-info" role="alert">
                        "This approved plan never reached the network: no transaction was submitted or seen in a mempool. Abandoning it frees its funding for a new review. The signed bytes stay on disk and are never broadcast by Wire."
                    </p>
                    <Actions>
                        <button class="btn btn-primary btn-sm" disabled=move || state.following.get()
                            on:click=move |_clicked| state.abandon_publication(abandon_id.clone())>"Yes, abandon it"</button>
                        <button class="btn btn-secondary btn-sm" on:click=move |_clicked| confirming.set(false)>"Keep it"</button>
                    </Actions>
                }.into_any()
            }
        }}
    }
}

fn kind_label(content: &Content) -> &'static str {
    match content {
        Content::Post { .. } => "Post / reply",
        Content::Nickname(_nickname) => "Nickname update · kind 05",
        Content::Avatar(_pixels) => "Avatar update · kind 06",
    }
}

fn topic_labels(content: &Content) -> Vec<String> {
    let Content::Post { wire, hashtags, .. } = content else {
        return Vec::new();
    };
    let mut spelled = Vec::new();
    if !wire.is_empty() {
        spelled.push(format!("/{wire}"));
    }
    for tag in hashtags {
        spelled.push(format!("#{tag}"));
    }
    spelled
}

fn frozen_text(content: &Content) -> String {
    match content {
        Content::Post { text, .. } => text.clone(),
        Content::Nickname(nickname) => nickname.clone(),
        Content::Avatar(_pixels) => String::new(),
    }
}

enum Target {
    Reply(String),
    Standalone,
}

fn target(content: &Content) -> Target {
    match content {
        Content::Post {
            addressed: Addressed::Thread(target),
            ..
        } => Target::Reply(target.to_display_hex()),
        _standalone => Target::Standalone,
    }
}

enum Frozen {
    Avatar(String),
    Malformed(String),
    Absent,
}

fn frozen_avatar(content: &Content) -> Frozen {
    let Content::Avatar(bytes) = content else {
        return Frozen::Absent;
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
