use leptos::prelude::*;
use wire_app_shared::{render_avatar_svg, render_identicon_svg, AuthorId};

#[component]
pub fn AvatarView(author: AuthorId, #[prop(default = 36)] size: u32) -> impl IntoView {
    let state = expect_context::<crate::state::AppState>();
    let svg = move || {
        state
            .public_identities
            .with(|identities| {
                identities
                    .get(&author.to_hex())
                    .and_then(|identity| identity.avatar.as_ref())
                    .and_then(|pixels| <&[u8; 512]>::try_from(pixels.as_slice()).ok())
                    .map(|pixels| render_avatar_svg(pixels, size))
            })
            .unwrap_or_else(|| render_identicon_svg(&author, size))
    };

    view! {
        <div
            class="avatar-container"
            style=format!("width: {size}px; height: {size}px;")
            inner_html=svg
        />
    }
}
