use crate::{components::ComposeBox, state::AppState};
use leptos::prelude::*;

#[component]
pub fn ComposePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let chain_name = move || state.active_chain.get().name();

    view! {
        <section class="page-container compose-page">
            <div class="page-nav-back">
                <a href="/" class="back-link">"← Back to feed"</a>
            </div>

            <div class="compose-page-header">
                <h1 class="page-title">"Publish a URMA Wire Record"</h1>
                <p class="page-subtitle">
                    "This post will be published to " <strong>{chain_name}</strong>
                    ". UTF-8 text is authenticated with the author's BIP 340 key and anchored in a single Taproot reveal transaction."
                </p>
            </div>

            <div class="compose-wrapper">
                <ComposeBox />
            </div>

            <div class="info-box">
                <h4>"URMA V0 Protocol Guarantees"</h4>
                <ul>
                    <li>"No article fragmentation: every post fits completely within the 32 KiB cap."</li>
                    <li>"Author identity is the x-only public key authorizing the Taproot spend."</li>
                    <li>"Zero detached application-level signatures or extraneous JSON wrappers."</li>
                    <li>"Posts become visible on the feed only after their first on-chain confirmation."</li>
                </ul>
            </div>
        </section>
    }
}
