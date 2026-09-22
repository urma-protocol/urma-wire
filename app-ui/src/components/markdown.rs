use crate::{
    platform::{tauri_call, use_platform_bridge},
    state::AppState,
};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::markdown::{segments, FetchedImage, Segment};

const MAX_DISPLAY_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGES_PER_POST: usize = 2;

pub fn load_autoload() -> Result<bool, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .ok_or("Browser storage unavailable")?
            .local_storage()
            .map_err(|_| "Image preference storage unavailable")?
            .ok_or("Image preference storage unavailable")?;
        return storage
            .get_item("wire-disable-external-images-v1")
            .map(|value| value.as_deref() != Some("true"))
            .map_err(|_| "Cannot read image preference".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(true)
}

pub fn save_autoload(enabled: bool) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .ok_or("Browser storage unavailable")?
            .local_storage()
            .map_err(|_| "Image preference storage unavailable")?
            .ok_or("Image preference storage unavailable")?;
        return storage
            .set_item(
                "wire-disable-external-images-v1",
                if enabled { "false" } else { "true" },
            )
            .map_err(|_| "Cannot save image preference".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[component]
pub fn Markdown(text: String) -> impl IntoView {
    let mut count = 0;
    view! { <div class="post-text markdown-content">
        {segments(&text).into_iter().map(|segment| match segment {
            Segment::Text(text) => view! { <span>{text}</span> }.into_any(),
            Segment::Image { url, alt } => {
                count += 1;
                if count > MAX_IMAGES_PER_POST {
                    view! { <span class="media-placeholder">{format!("Image: {alt} — display limit: 2 images per post")}</span> }.into_any()
                } else {
                    view! { <ExternalImage url= url alt=alt /> }.into_any()
                }
            }
        }).collect_view()}
    </div> }
}

#[component]
fn ExternalImage(url: String, alt: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let platform = use_platform_bridge();
    let capable = platform.can_autoload_images();
    let secure_context = {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|window| window.location().protocol().ok())
                .as_deref()
                == Some("https:")
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    };
    let unavailable = if capable {
        wire_app_shared::markdown::image_url(&url).err()
    } else {
        Some(wire_app_shared::markdown::browser_image_error(
            &url,
            secure_context,
        ))
    };
    let permitted = unavailable.is_none();
    let data = RwSignal::new(Option::<FetchedImage>::None);
    let message = RwSignal::new(unavailable);
    let loading = RwSignal::new(false);
    let requested = RwSignal::new(false);
    let attempted = RwSignal::new(false);
    let generation = RwSignal::new(0u64);
    let mounted_epoch = state.media_epoch.get_untracked();
    let url = StoredValue::new(url);
    Effect::new(move |_| {
        let epoch = state.media_epoch.get();
        if epoch != mounted_epoch {
            data.set(None);
            requested.set(false);
            attempted.set(true);
            generation.update(|value| *value += 1);
        }
    });
    Effect::new(move |_| {
        let wants = requested.get() || (state.autoload_images.get() && !attempted.get());
        if !permitted || !wants || loading.get() || state.media_inflight.get() >= 2 {
            return;
        }
        if state.media_bytes.get_untracked() >= MAX_DISPLAY_BYTES {
            message.set(Some(
                "Displayed media reached 64 MiB. Clear media in Wallet settings to load more."
                    .into(),
            ));
            requested.set(false);
            attempted.set(true);
            return;
        }
        loading.set(true);
        attempted.set(true);
        requested.set(false);
        message.set(None);
        let epoch = state.media_epoch.get_untracked();
        let request_generation = generation.get_untracked();
        state.media_inflight.update(|count| *count += 1);
        let url = url.get_value();
        leptos::task::spawn_local(async move {
            let result =
                tauri_call::<FetchedImage>("fetch_external_image", &json!({"url": url})).await;
            state
                .media_inflight
                .update(|count| *count = count.saturating_sub(1));
            let _ = loading.try_set(false);
            if generation.try_get() != Some(request_generation)
                || state.media_epoch.get_untracked() != epoch
            {
                return;
            }
            match result {
                Ok(image)
                    if state
                        .media_bytes
                        .get_untracked()
                        .saturating_add(image.display_bytes())
                        <= MAX_DISPLAY_BYTES =>
                {
                    state
                        .media_bytes
                        .update(|bytes| *bytes += image.display_bytes());
                    let _ = data.try_set(Some(image));
                }
                Ok(_) => {
                    let _ = message.try_set(Some(
                        "Image exceeds remaining display budget. Clear media in Wallet settings."
                            .into(),
                    ));
                }
                Err(error) => {
                    let _ = message.try_set(Some(error));
                }
            }
        });
    });
    let load = move |_| requested.set(true);
    on_cleanup(move || {
        if let Some(Some(previous)) = data.try_get() {
            state
                .media_bytes
                .update(|bytes| *bytes = bytes.saturating_sub(previous.display_bytes()));
        }
    });
    let placeholder_alt = alt.clone();
    // A loaded image stands alone: the placeholder, and with it the button, exist
    // only while there is nothing to show. Loading stays manual only when the
    // reader turned autoload off, or when a failure left something to retry.
    let shown = move || data.get().is_some();
    let offers_load =
        move || permitted && !loading.get() && (!state.autoload_images.get() || message.get().is_some());
    view! {
        <span class="external-image">
            {move || data.get().map(|image| view! { <img src=image.data_url alt=alt.clone() /> })}
            {move || (!shown()).then(|| view! {
                <span class="media-placeholder">
                    <span>{placeholder_alt.clone()}</span>
                    {move || if loading.get() { "Loading checked image…" } else { "External image" }}
                    {move || message.get().map(|message| view! { <span>{message}</span> })}
                    {move || offers_load().then(|| view! {
                        <button class="btn btn-secondary btn-sm" on:click=load>
                            {move || if message.get().is_some() { "Retry image" } else { "Load image" }}
                        </button>
                    })}
                </span>
            })}
        </span>
    }
}
