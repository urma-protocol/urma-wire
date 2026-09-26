use crate::state::Said;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CustomAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::Signal;
use leptos::prelude::{component, view, Children};

#[component]
pub fn PageHeader(
    eyebrow: &'static str,
    title: &'static str,
    accent: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <header class="page-header">
            <div>
                <span class="eyebrow">{eyebrow}</span>
                <h1 class="page-title">{title}" "<span class="accent-text">{accent}</span></h1>
            </div>
            <p class="page-subtitle">{children()}</p>
        </header>
    }
}

#[component]
pub fn SettingsGroup(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="settings-group">
            <h3 class="group-title">{title}</h3>
            {children()}
        </div>
    }
}

#[component]
pub fn DescribedGroup(
    title: &'static str,
    description: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="settings-group">
            <h3 class="group-title">{title}</h3>
            <p class="group-desc">{description}</p>
            {children()}
        </div>
    }
}

#[component]
pub fn Actions(children: Children) -> impl IntoView {
    view! { <div class="actions">{children()}</div> }
}

#[component]
pub fn Facts(children: Children) -> impl IntoView {
    view! { <dl class="facts">{children()}</dl> }
}

#[component]
pub fn Fact(
    label: &'static str,
    #[prop(default = false)] wide: bool,
    children: Children,
) -> impl IntoView {
    let class = match wide {
        true => "fact wide",
        false => "fact",
    };
    view! {
        <div class=class>
            <dt>{label}</dt>
            <dd>{children()}</dd>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Live,
    Idle,
    Trouble,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Tone::Live => "status live",
            Tone::Idle => "status idle",
            Tone::Trouble => "status trouble",
        }
    }
}

#[component]
pub fn Status(tone: Tone, text: &'static str) -> impl IntoView {
    view! {
        <span class=tone.class()>
            <span class="status-dot" aria-hidden="true"></span>
            {text}
        </span>
    }
}

#[component]
pub fn Notice(message: Signal<Said>) -> impl IntoView {
    view! {
        {move || match message.get() {
            Said::Something(message) => view! { <div class="alert alert-info" role="status">{message}</div> }.into_any(),
            Said::Nothing => ().into_any(),
        }}
    }
}

#[component]
pub fn Warning(message: Signal<Said>) -> impl IntoView {
    view! {
        {move || match message.get() {
            Said::Something(message) => view! { <div class="alert alert-error" role="alert">{message}</div> }.into_any(),
            Said::Nothing => ().into_any(),
        }}
    }
}

#[component]
pub fn Empty(heading: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="empty-state">
            <h3>{heading}</h3>
            <p>{children()}</p>
        </div>
    }
}

#[component]
pub fn Missing(heading: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="empty-state not-found">
            <svg class="empty-mark" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="11" cy="11" r="7"/>
                <path d="m16.5 16.5 4.5 4.5M8 11h6"/>
            </svg>
            <h3>{heading}</h3>
            {children()}
            <a href="/" class="btn btn-secondary">"Return to feed"</a>
        </div>
    }
}

#[component]
pub fn BackToFeed() -> impl IntoView {
    view! {
        <div class="page-nav-back">
            <a href="/" class="back-link">"← Back to feed"</a>
        </div>
    }
}

#[component]
pub fn Stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="grid-stat">
            <span class="stat-label">{label}</span>
            <span class="stat-value">{value}</span>
        </div>
    }
}
