use crate::components::Warning;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::{component, view};
use wire_app_core::model::Chain;
use wire_app_core::publication::{Kind, Publication};

#[component]
pub fn PublicationPanel(kind: Kind) -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <div class="publication-panel">
            {move || match stage(state, kind) {
                Stage::Quote(publication) => view! { <Quote publication=publication /> }.into_any(),
                Stage::Sending => view! {
                    <p class="publish-sending" role="status">"Publishing…"</p>
                }.into_any(),
                Stage::Confirming => view! {
                    <p class="publish-sending" role="status">"Sent. Waiting for a Litecoin block…"</p>
                }.into_any(),
                Stage::Published => view! {
                    <p class="publish-done" role="status">"Published."</p>
                }.into_any(),
                Stage::Stalled(publication) => view! { <Stalled publication=publication /> }.into_any(),
                Stage::Hidden => ().into_any(),
            }}
            {move || match concerns(state, kind) {
                true => view! { <Warning message=state.publication_error.into() /> }.into_any(),
                false => ().into_any(),
            }}
        </div>
    }
}

pub enum Stage {
    Quote(Publication),
    Sending,
    Confirming,
    Published,
    Stalled(Publication),
    Hidden,
}

pub fn staged(publication: Publication, following: bool) -> Stage {
    if publication.complete {
        return Stage::Published;
    }
    if !publication.approved {
        return Stage::Quote(publication);
    }
    if !following {
        return Stage::Stalled(publication);
    }
    match publication.confirmed + publication.mempool >= 2 {
        true => Stage::Confirming,
        false => Stage::Sending,
    }
}

fn stage(state: AppState, kind: Kind) -> Stage {
    let Some(publication) = state.publication.get() else {
        return Stage::Hidden;
    };
    if publication.content.kind() != kind {
        return Stage::Hidden;
    }
    staged(publication, state.following.get())
}

fn concerns(state: AppState, kind: Kind) -> bool {
    let Some(publication) = state.publication.get() else {
        return kind == Kind::Post;
    };
    publication.content.kind() == kind
}

#[component]
fn Quote(publication: Publication) -> impl IntoView {
    let state = expect_context::<AppState>();
    let id = publication.id.clone();
    view! {
        <div class="publish-quote" role="dialog" aria-label="Publication cost">
            <span>"Publish for "<strong>{Chain::Litecoin.amount(publication.fee)}</strong></span>
            <button class="btn btn-primary btn-sm" disabled=move || state.following.get()
                on:click=move |_clicked| state.follow(id.clone(), true)>"Confirm"</button>
        </div>
    }
}

#[component]
fn Stalled(publication: Publication) -> impl IntoView {
    let state = expect_context::<AppState>();
    let id = publication.id.clone();
    let abandon_id = publication.id.clone();
    view! {
        <div class="publish-quote" role="alert">
            <span>"Publication paused"</span>
            <button class="btn btn-primary btn-sm"
                on:click=move |_clicked| state.follow(id.clone(), false)>"Retry"</button>
            {match publication.abandonable {
                true => view! {
                    <button class="btn btn-secondary btn-sm"
                        on:click=move |_clicked| state.abandon_publication(abandon_id.clone())>"Abandon"</button>
                }.into_any(),
                false => ().into_any(),
            }}
        </div>
    }
}
