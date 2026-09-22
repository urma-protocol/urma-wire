use crate::components::said::Said;
use leptos::prelude::ClassAttribute;
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
    subtitle: &'static str,
) -> impl IntoView {
    view! {
        <div>
            <span class="eyebrow">{eyebrow}</span>
            <h1 class="page-title">{title}" "<span class="accent-text">{accent}</span></h1>
        </div>
        <p class="page-subtitle">{subtitle}</p>
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
pub fn Missing(icon: &'static str, heading: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="empty-state not-found">
            <span class="empty-icon">{icon}</span>
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
pub fn DetailRow(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="wallet-row">
            <span class="label">{label}</span>
            {children()}
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
