use crate::components::AvatarView;
use crate::components::BackToFeed;
use crate::components::Missing;
use crate::components::PostCard;
use crate::limits::PROFILE_AVATAR_PIXELS;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::With;
use leptos::prelude::{component, view};
use leptos_router::hooks::use_params_map;
use tracing::warn;
use wire_app_shared::AuthorId;
use wire_app_shared::Post;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let params = use_params_map();
    let named = move || {
        let Some(hex) = params.get().get("author") else {
            return Named::Unreadable;
        };
        match AuthorId::from_hex(&hex) {
            Ok(author) => Named::Author(author),
            Err(error) => {
                warn!(error = %error, "profile link carries an unreadable author key");
                Named::Unreadable
            }
        }
    };
    view! {
        <section class="page-container profile-page">
            <BackToFeed />
            {move || match named() {
                Named::Author(author) => view! { <Dossier author=author /> }.into_any(),
                Named::Unreadable => view! {
                    <Missing icon="⚠️" heading="Invalid Author Key">
                        <p>"The specified author public key is not a valid 32-byte BIP 340 hex key."</p>
                    </Missing>
                }.into_any(),
            }}
        </section>
    }
}

enum Named {
    Author(AuthorId),
    Unreadable,
}

#[component]
fn Dossier(author: AuthorId) -> impl IntoView {
    let state = expect_context::<AppState>();
    let posts = move || written_by(state, author);
    view! {
        <div class="profile-card">
            <div class="profile-header">
                <AvatarView author=author size=PROFILE_AVATAR_PIXELS />
                <div class="profile-info">
                    <h2 class="profile-name">{move || state.display_name(author)}</h2>
                    <div class="profile-fingerprint-badge">
                        "🔑 Key fingerprint: " <strong>{author.fingerprint()}</strong>
                    </div>
                    <div class="profile-key-full">
                        <span class="label">"BIP 340 public key (x-only):"</span>
                        <code class="key-code">{author.to_hex()}</code>
                    </div>
                    <div class="profile-avatar-status">
                        <span class="badge badge-neutral">{move || avatar_state(state, author)}</span>
                    </div>
                </div>
            </div>
        </div>
        <div class="author-stream-section">
            <h3 class="section-title">
                {move || format!(
                    "Confirmed posts by author on {} ({})",
                    state.active_chain.get().network_name(),
                    posts().len(),
                )}
            </h3>
            {move || {
                let written = posts();
                match written.is_empty() {
                    true => view! {
                        <div class="empty-state">
                            <p>"No confirmed posts recorded by this author on this chain."</p>
                        </div>
                    }.into_any(),
                    false => view! {
                        <div class="posts-list">
                            {written.into_iter().map(|post| view! { <PostCard post=post /> }).collect_view()}
                        </div>
                    }.into_any(),
                }
            }}
        </div>
    }
}

fn written_by(state: AppState, author: AuthorId) -> Vec<Post> {
    let mut written = Vec::new();
    for post in state.feed.get() {
        if post.author == author {
            written.push(post);
        }
    }
    written
}

fn avatar_state(state: AppState, author: AuthorId) -> &'static str {
    let confirmed = state.public_identities.with(|identities| {
        let Some(identity) = identities.get(&author.to_hex()) else {
            return false;
        };
        let Some(avatar) = identity.avatar.as_ref() else {
            return false;
        };
        !avatar.is_empty()
    });
    match confirmed {
        true => "Confirmed pixel-art avatar",
        false => "Local fallback identicon",
    }
}
