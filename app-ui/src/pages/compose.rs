use crate::components::BackToFeed;
use crate::components::ComposeBox;
use crate::components::PageHeader;
use crate::components::PublicationPanel;
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
            <PageHeader eyebrow="YOUR VOICE. ON RECORD." title="Make your" accent="mark.">
                "Write, review the exact cost, then publish to " <strong>{chain_name}</strong> "."
            </PageHeader>
            <ComposeBox />
            <PublicationPanel />
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
