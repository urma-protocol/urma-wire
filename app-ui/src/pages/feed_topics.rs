use leptos::prelude::*;
use wire_app_shared::Chain;

#[component]
pub fn FeedTopics(
    chain: RwSignal<Chain>,
    wire_filter: RwSignal<String>,
    hashtag_filter: RwSignal<String>,
    followed: RwSignal<Vec<String>>,
    only_followed: RwSignal<bool>,
) -> impl IntoView {
    let message = RwSignal::new(String::new());
    Effect::new(move |_| {
        let key = format!(
            "wire:topics:v1:{}:{}",
            chain.get().code(),
            chain.get().network_name()
        );
        let loaded = storage()
            .and_then(|storage| {
                storage
                    .get_item(&key)
                    .map_err(|_| "Cannot read local follows".to_string())
            })
            .and_then(|value| {
                value
                    .map(|value| {
                        serde_json::from_str::<Vec<String>>(&value).map_err(|e| e.to_string())
                    })
                    .transpose()
            });
        match loaded {
            Ok(value) => followed.set(
                value
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|label| valid(label))
                    .take(128)
                    .collect(),
            ),
            Err(error) => {
                followed.set(Vec::new());
                message.set(error);
            }
        }
        wire_filter.set(String::new());
        hashtag_filter.set(String::new());
        only_followed.set(false);
    });
    view! {
        <div class="feed-topics">
            <div class="publication-actions">
                <button class="btn btn-secondary btn-sm" on:click=move |_| { wire_filter.set(String::new()); hashtag_filter.set(String::new()); only_followed.set(false); }>"Global feed"</button>
                <button class="btn btn-secondary btn-sm" on:click=move |_| { wire_filter.set(String::new()); hashtag_filter.set(String::new()); only_followed.set(true); }>"Following"</button>
            </div>
            <TopicFilter name="wire" prefix='/' filter=wire_filter chain=chain followed=followed only_followed=only_followed message=message />
            <TopicFilter name="hashtag" prefix='#' filter=hashtag_filter chain=chain followed=followed only_followed=only_followed message=message />
            <p class="action-desc">{move || if only_followed.get() { "Showing any followed wire or hashtag" } else { "Optional filters match both when set. Showing up to 100 matches from indexed history." }}</p>
            <div class="topic-labels">{move || followed.get().into_iter().map(|label| { let selected = label.clone(); view! { <button class="btn btn-secondary btn-sm" on:click=move |_| {
                if let Some(wire) = selected.strip_prefix('/') { wire_filter.set(wire.into()); }
                else if let Some(tag) = selected.strip_prefix('#') { hashtag_filter.set(tag.into()); }
                only_followed.set(false);
            }>{label}</button> } }).collect_view()}</div>
            <p role="status">{move || message.get()}</p>
        </div>
    }
}

#[component]
fn TopicFilter(
    name: &'static str,
    prefix: char,
    filter: RwSignal<String>,
    chain: RwSignal<Chain>,
    followed: RwSignal<Vec<String>>,
    only_followed: RwSignal<bool>,
    message: RwSignal<String>,
) -> impl IntoView {
    let label = move || format!("{prefix}{}", filter.get());
    let toggle = move |_| {
        let label = label();
        if !valid(&label) {
            message.set("Enter /wire or #hashtag: 1–32 lowercase letters, digits, - or _.".into());
            return;
        }
        let mut labels = followed.get();
        if labels.contains(&label) {
            labels.retain(|item| item != &label);
        } else if labels.len() < 128 {
            labels.push(label.clone());
            labels.sort();
        } else {
            message.set("Local follow limit reached (128).".into());
            return;
        }
        let key = format!(
            "wire:topics:v1:{}:{}",
            chain.get().code(),
            chain.get().network_name()
        );
        match serde_json::to_string(&labels)
            .map_err(|e| e.to_string())
            .and_then(|json| {
                storage()?
                    .set_item(&key, &json)
                    .map_err(|_| "Cannot save local follows".to_string())
            }) {
            Ok(()) => {
                followed.set(labels);
                message.set("Saved locally. No transaction or fee.".into());
            }
            Err(error) => message.set(error),
        }
    };
    view! {
        <div class="topic-filter">
            <label>{format!("Filter by {name} (optional)")}<input class="input" placeholder=if prefix == '/' { "/rosint" } else { "#investigation" } prop:value=move || filter.get() on:input=move |ev| {
                only_followed.set(false);
                let value = event_target_value(&ev);
                let value = value.trim();
                filter.set(value.strip_prefix(prefix).unwrap_or(value).to_ascii_lowercase());
            } /></label>
            <button class="btn btn-secondary btn-sm" on:click=move |_| { filter.set(String::new()); only_followed.set(false); }>{format!("Clear {name}")}</button>
            <button class="btn btn-secondary btn-sm" disabled=move || !valid(&label()) on:click=toggle>{move || format!("{} {name}", if followed.get().contains(&label()) { "Unfollow" } else { "Follow" })}</button>
        </div>
    }
}

fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("No browser storage")?
        .local_storage()
        .map_err(|_| "Local storage is unavailable")?
        .ok_or("Local storage is unavailable".into())
}

fn valid(label: &str) -> bool {
    let bytes = label.as_bytes();
    (2..=33).contains(&bytes.len())
        && matches!(bytes[0], b'/' | b'#')
        && (bytes[1].is_ascii_lowercase() || bytes[1].is_ascii_digit())
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
