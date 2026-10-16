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
use wire_app_core::publication::Publication;

#[component]
pub fn PublicationPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <div class="publication-panel">
            {move || match stage(state) {
                Stage::Quote(publication) => view! { <Quote publication=publication /> }.into_any(),
                Stage::Sending => view! {
                    <p class="publish-sending" role="status">"Publishing…"</p>
                }.into_any(),
                Stage::Stalled(publication) => view! { <Stalled publication=publication /> }.into_any(),
                Stage::Hidden => ().into_any(),
            }}
            <Warning message=state.publication_error.into() />
        </div>
    }
}

pub enum Stage {
    Quote(Publication),
    Sending,
    Stalled(Publication),
    Hidden,
}

pub fn staged(publication: Publication, following: bool) -> Stage {
    let submitted = publication.confirmed + publication.mempool >= 2;
    if publication.complete || (publication.approved && submitted) {
        return Stage::Hidden;
    }
    if !publication.approved {
        return Stage::Quote(publication);
    }
    match following {
        true => Stage::Sending,
        false => Stage::Stalled(publication),
    }
}

fn stage(state: AppState) -> Stage {
    let Some(publication) = state.publication.get() else {
        return Stage::Hidden;
    };
    staged(publication, state.following.get())
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
