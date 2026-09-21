use crate::{
    components::{AvatarView, PostCard},
    state::AppState,
};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use wire_app_shared::AuthorId;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();

    let author_hex_str = move || params.read().get("author").unwrap_or_default();

    let parsed_author = move || {
        let s = author_hex_str();
        AuthorId::from_hex(&s).ok()
    };

    let author_posts = move || {
        if let Some(author) = parsed_author() {
            state
                .feed
                .get()
                .into_iter()
                .filter(|p| p.author == author)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    view! {
        <section class="page-container profile-page">
            <div class="page-nav-back">
                <a href="/" class="back-link">"← Back to feed"</a>
            </div>

            {move || {
                match parsed_author() {
                    Some(author) => {
                        let posts = author_posts();
                        let display_name = state.display_name(author);
                        let full_hex = author.to_hex();
                        let fingerprint = author.fingerprint();
                        let chain_name = state.active_chain.get().network_name();

                        view! {
                            <div class="profile-card">
                                <div class="profile-header">
                                    <AvatarView author=author size=72 />
                                    <div class="profile-info">
                                        <h2 class="profile-name">{display_name}</h2>
                                        <div class="profile-fingerprint-badge">
                                            "🔑 Key fingerprint: " <strong>{fingerprint}</strong>
                                        </div>
                                        <div class="profile-key-full">
                                            <span class="label">"BIP 340 public key (x-only):"</span>
                                            <code class="key-code">{full_hex}</code>
                                        </div>
                                        <div class="profile-avatar-status">
                                            <span class="badge badge-neutral">
                                                {if state.public_identities.with(|identities| identities.get(&author.to_hex()).is_some_and(|identity| identity.avatar.is_some())) { "Confirmed pixel-art avatar" } else { "Local fallback identicon" }}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div class="author-stream-section">
                                <h3 class="section-title">
                                    "Confirmed posts by author on " {chain_name} " (" {posts.len()} ")"
                                </h3>

                                {if posts.is_empty() {
                                    view! {
                                        <div class="empty-state">
                                            <p>"No confirmed posts recorded by this author on this chain."</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="posts-list">
                                            {posts.into_iter().map(|p| {
                                                view! { <PostCard post=p /> }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                    None => {
                        view! {
                            <div class="empty-state not-found">
                                <span class="empty-icon">"⚠️"</span>
                                <h3>"Invalid Author Key"</h3>
                                <p>"The specified author public key is not a valid 32-byte BIP 340 hex key."</p>
                                <a href="/" class="btn btn-secondary">"Return to feed"</a>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </section>
    }
}
