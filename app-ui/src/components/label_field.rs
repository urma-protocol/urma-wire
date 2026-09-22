use crate::data::topics::Suggestion;
use crate::state::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::Callable;
use leptos::prelude::Callback;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, event_target_value, view};
use wire_app_shared::topic_labels::Kind;

#[component]
pub fn LabelField(
    label: &'static str,
    placeholder: &'static str,
    kind: Kind,
    value: RwSignal<String>,
    single: bool,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let highlighted = RwSignal::new(0usize);
    let open = RwSignal::new(false);

    let offered = move || {
        let typed = value.get();
        let word = match single {
            true => typed.clone(),
            false => trailing_word(&typed),
        };
        state
            .topics
            .get()
            .suggestions(kind, &word, &state.seen_topics())
    };

    let accept = move |suggestion: Suggestion| {
        let typed = value.get_untracked();
        value.set(match single {
            true => suggestion.label.clone(),
            false => replace_trailing(&typed, &suggestion.label),
        });
        highlighted.set(0);
        open.set(false);
    };
    let navigate = move |event: leptos::ev::KeyboardEvent| {
        walk(&event, offered(), highlighted, open, accept);
    };

    view! {
        <label class="label-field">
            {label}
            <div class="label-field-input">
                <span class="label-field-mark" aria-hidden="true">{kind.mark().to_string()}</span>
                <input placeholder=placeholder autocomplete="off"
                    prop:value=move || value.get()
                    on:input=move |event| { value.set(event_target_value(&event)); open.set(true); highlighted.set(0); }
                    on:focus=move |_focused| open.set(true)
                    on:keydown=navigate />
            </div>
            {move || {
                let ranked = offered();
                if !open.get() || ranked.is_empty() {
                    return ().into_any();
                }
                view! { <Offered ranked=ranked highlighted=highlighted accept=Callback::new(accept) /> }.into_any()
            }}
        </label>
    }
}

#[component]
fn Offered(
    ranked: Vec<Suggestion>,
    highlighted: RwSignal<usize>,
    accept: Callback<Suggestion>,
) -> impl IntoView {
    view! {
        <ul class="label-suggestions" role="listbox">
            {ranked.into_iter().enumerate().map(|(at, suggestion)| {
                let chosen = suggestion.clone();
                let spelled = suggestion.spelled();
                let followed = suggestion.followed;
                view! {
                    <li role="option" aria-selected=move || (highlighted.get() == at).to_string()
                        class=move || match highlighted.get() == at { true => "label-suggestion active", false => "label-suggestion" }
                        on:mouseenter=move |_entered| highlighted.set(at)
                        on:mousedown=move |event| { event.prevent_default(); accept.run(chosen.clone()); }>
                        <span>{spelled}</span>
                        {match followed {
                            true => view! { <span class="label-suggestion-followed">"followed"</span> }.into_any(),
                            false => ().into_any(),
                        }}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

fn walk(
    event: &leptos::ev::KeyboardEvent,
    ranked: Vec<Suggestion>,
    highlighted: RwSignal<usize>,
    open: RwSignal<bool>,
    accept: impl Fn(Suggestion),
) {
    if ranked.is_empty() {
        return;
    }
    match stroke(&event.key()) {
        Stroke::Down => {
            event.prevent_default();
            open.set(true);
            highlighted.set(stepped(highlighted.get_untracked(), ranked.len(), 1));
        }
        Stroke::Up => {
            event.prevent_default();
            open.set(true);
            highlighted.set(stepped(highlighted.get_untracked(), ranked.len(), -1));
        }
        Stroke::Accept => {
            if !open.get_untracked() {
                return;
            }
            let Some(chosen) = ranked.get(highlighted.get_untracked()) else {
                return;
            };
            event.prevent_default();
            accept(chosen.clone());
        }
        Stroke::Dismiss => open.set(false),
        Stroke::Typing => (),
    }
}

enum Stroke {
    Down,
    Up,
    Accept,
    Dismiss,
    Typing,
}

fn stroke(key: &str) -> Stroke {
    match key {
        "ArrowDown" => Stroke::Down,
        "ArrowUp" => Stroke::Up,
        "Enter" | "Tab" => Stroke::Accept,
        "Escape" => Stroke::Dismiss,
        _ => Stroke::Typing,
    }
}

fn stepped(at: usize, count: usize, by: isize) -> usize {
    let last = count - 1;
    match by {
        1 => match at >= last {
            true => 0,
            false => at + 1,
        },
        _ => match at {
            0 => last,
            _ => at - 1,
        },
    }
}

fn trailing_word(typed: &str) -> String {
    let Some(word) = typed.split_whitespace().next_back() else {
        return String::new();
    };
    match typed.ends_with(char::is_whitespace) {
        true => String::new(),
        false => word.to_owned(),
    }
}

fn replace_trailing(typed: &str, label: &str) -> String {
    let mut words: Vec<String> = typed.split_whitespace().map(str::to_owned).collect();
    if !typed.ends_with(char::is_whitespace) {
        words.pop();
    }
    words.push(label.to_owned());
    let mut spelled = words.join(" ");
    spelled.push(' ');
    spelled
}
