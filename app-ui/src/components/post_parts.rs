use crate::components::ComposeBox;
use crate::state::addressing::Speaking;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::With;
use leptos::prelude::{component, view};
use wire_app_core::model::{Post, Txid};

#[component]
pub fn ReplyToggle(post: Post) -> impl IntoView {
    let state = expect_context::<AppState>();
    let txid = post.txid;
    view! {
        <button type="button"
            class=move || match state.speaking.with(|speaking| speaking.replies_to(txid)) {
                true => "post-action replying",
                false => "post-action",
            }
            aria-expanded=move || state.speaking.with(|speaking| speaking.replies_to(txid)).to_string()
            on:click=move |_clicked| state.toggle_reply_to(post.clone())>
            "↳ Reply"
        </button>
    }
}

#[component]
pub fn ThreadCount(txid: Txid, count: usize) -> impl IntoView {
    view! {
        <a href=format!("/post/{}", txid.to_display_hex()) class="post-action" title="Open thread">
            {count}" "{plural(count)}
        </a>
    }
}

#[component]
pub fn InlineReply(post: Post) -> impl IntoView {
    let state = expect_context::<AppState>();
    let txid = post.txid;
    view! {
        {move || match state.speaking.with(|speaking| speaking.replies_to(txid)) {
            true => view! {
                <div class="post-inline-reply">
                    <ComposeBox speaking=Speaking::InReplyTo(post.clone()) />
                </div>
            }.into_any(),
            false => ().into_any(),
        }}
    }
}

fn plural(count: usize) -> &'static str {
    match count {
        1 => "reply",
        _ => "replies",
    }
}
