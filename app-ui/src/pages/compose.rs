use crate::components::BackToFeed;
use crate::components::ComposeBox;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::IntoView;
use leptos::prelude::{component, view};

#[component]
pub fn ComposePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let chain_name = move || state.active_chain.get().network_name();

    view! {
        <section class="page-container compose-page">
            <BackToFeed />

            <div class="compose-page-header">
                <span class="eyebrow">"YOUR VOICE. ON RECORD."</span>
                <h1 class="page-title">"Make your " <span class="accent-text">"mark."</span></h1>
                <p class="page-subtitle">
                    "Write, review the exact cost, then publish to " <strong>{chain_name}</strong> "."
                </p>
            </div>

            <div class="compose-wrapper">
                <ComposeBox />
            </div>

            <div class="info-box">
                <h4>"Before you publish"</h4>
                <ul>
                    <li>"Your text is public. Only publish what you intend to share."</li>
                    <li>"Your identity signs the record and pays the displayed network fee."</li>
                    <li>"The feed shows your post after blockchain confirmation."</li>
                </ul>
            </div>
        </section>
    }
}
