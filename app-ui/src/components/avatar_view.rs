use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::InnerHtmlAttribute;
use leptos::prelude::IntoView;
use leptos::prelude::StyleAttribute;
use leptos::prelude::With;
use leptos::prelude::{component, view};
use std::collections::BTreeMap;
use tracing::warn;
use wire_app_core::avatar::{render_avatar_svg, render_identicon_svg};
use wire_app_core::feed::PublicIdentity;
use wire_app_core::model::AuthorId;

#[component]
pub fn AvatarView(author: AuthorId, #[prop(default = 36)] size: u32) -> impl IntoView {
    let state = expect_context::<AppState>();
    let svg = move || {
        state
            .public_identities
            .with(|identities| drawn(identities, author, size))
    };
    view! {
        <div
            class="avatar-container"
            style=format!("width: {size}px; height: {size}px;")
            inner_html=svg
        />
    }
}

fn drawn(identities: &BTreeMap<String, PublicIdentity>, author: AuthorId, size: u32) -> String {
    let Some(identity) = identities.get(&author.to_hex()) else {
        return render_identicon_svg(&author, size);
    };
    let Some(pixels) = identity.avatar.as_ref() else {
        return render_identicon_svg(&author, size);
    };
    match <&[u8; 512]>::try_from(pixels.as_slice()) {
        Ok(pixels) => render_avatar_svg(pixels, size),
        Err(error) => {
            warn!(error = %error, "confirmed avatar is not 512 packed bytes");
            render_identicon_svg(&author, size)
        }
    }
}
