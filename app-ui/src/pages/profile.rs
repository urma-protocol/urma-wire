use crate::components::AvatarView;
use crate::components::BackToFeed;
use crate::components::Empty;
use crate::components::Fact;
use crate::components::Facts;
use crate::components::Missing;
use crate::components::PostCard;
use crate::components::PublicationPanel;
use crate::config::PROFILE_AVATAR_PIXELS;
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
use wire_app_core::model::{AuthorId, Post};
use wire_app_core::publication::Kind;

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
                    <Missing heading="Invalid author key">
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
    let network = move || state.active_chain.get().network_name();
    view! {
        <div class="profile-card">
            <div class="profile-header">
                <AvatarView author=author size=PROFILE_AVATAR_PIXELS />
                <div class="profile-info">
                    <h2 class="profile-name">{move || state.display_name(author)}</h2>
                    <Facts>
                        <Fact label="Fingerprint"><span class="badge badge-neutral">{author.fingerprint()}</span></Fact>
                        <Fact label="Avatar">{move || avatar_state(state, author)}</Fact>
                        <Fact label="BIP 340 public key (x-only)" wide=true>
                            <code class="key-code">{author.to_hex()}</code>
                        </Fact>
                    </Facts>
                </div>
            </div>
        </div>
        <PublicationPanel kind=Kind::Post />
        <div class="author-stream-section">
            <h3 class="section-title">
                {move || format!("Confirmed posts on {}", network())}
                <small>{move || posts().len()}</small>
            </h3>
            {move || {
                let written = posts();
                match written.is_empty() {
                    true => view! {
                        <Empty heading="No confirmed posts">
                            "This author has no confirmed record on this chain within the indexed history."
                        </Empty>
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
