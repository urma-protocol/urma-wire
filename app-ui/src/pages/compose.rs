use crate::{components::ComposeBox, state::AppState};
use leptos::prelude::*;

#[component]
pub fn ComposePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let chain_name = move || state.active_chain.get().network_name();

    view! {
        <section class="page-container compose-page">
            <div class="page-nav-back">
                <a href="/" class="back-link">"← Back to feed"</a>
            </div>

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
