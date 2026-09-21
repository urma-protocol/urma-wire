use leptos::prelude::*;
use wire_app_shared::{render_identicon_svg, AuthorId};

#[component]
pub fn AvatarView(author: AuthorId, #[prop(default = 36)] size: u32) -> impl IntoView {
    let svg = move || render_identicon_svg(&author, size);

    view! {
        <div
            class="avatar-container"
            style=format!("width: {size}px; height: {size}px;")
            inner_html=svg
        />
    }
}
