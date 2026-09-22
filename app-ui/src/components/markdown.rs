use crate::components::budget::Budget;
use crate::components::budget::Room;
use crate::components::Said;
use crate::data::media;
use crate::limits::autoload_images_when_unset;
use crate::limits::CONCURRENT_IMAGE_FETCHES;
use crate::limits::IMAGE_PREFERENCE_KEY;
use crate::limits::MAX_IMAGES_PER_POST;
use crate::platform::use_platform_bridge;
use crate::state::session::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::Effect;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GetValue;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::StoredValue;
use leptos::prelude::Update;
use leptos::prelude::{component, on_cleanup, view};
use tracing::warn;
use wire_app_shared::markdown::browser_image_error;
use wire_app_shared::markdown::image_url;
use wire_app_shared::markdown::segments;
use wire_app_shared::markdown::FetchedImage;
use wire_app_shared::markdown::Segment;

pub enum Preference {
    Autoload,
    Withhold,
    Unreadable(String),
}

impl Preference {
    pub fn wanted(&self) -> bool {
        match self {
            Preference::Autoload => true,
            Preference::Withhold => false,
            Preference::Unreadable(_) => autoload_images_when_unset(),
        }
    }

    pub fn complaint(&self) -> Said {
        match self {
            Preference::Autoload => Said::Nothing,
            Preference::Withhold => Said::Nothing,
            Preference::Unreadable(reason) => Said::Something(reason.clone()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
enum Kept {
    Storage(web_sys::Storage),
    Unavailable(String),
}

#[cfg(target_arch = "wasm32")]
fn preference_storage() -> Kept {
    let Some(window) = web_sys::window() else {
        return Kept::Unavailable("Browser storage unavailable".to_owned());
    };
    let stored = match window.local_storage() {
        Ok(stored) => stored,
        Err(error) => {
            warn!(error = ?error, "image preference storage unavailable");
            return Kept::Unavailable("Image preference storage unavailable".to_owned());
        }
    };
    let Some(storage) = stored else {
        return Kept::Unavailable("Image preference storage unavailable".to_owned());
    };
    Kept::Storage(storage)
}

#[cfg(target_arch = "wasm32")]
pub fn stored_autoload() -> Preference {
    let storage = match preference_storage() {
        Kept::Storage(storage) => storage,
        Kept::Unavailable(reason) => return Preference::Unreadable(reason),
    };
    match storage.get_item(IMAGE_PREFERENCE_KEY) {
        Ok(Some(value)) => match value.as_str() {
            "true" => Preference::Withhold,
            _ => Preference::Autoload,
        },
        Ok(None) => Preference::Withhold,
        Err(error) => {
            warn!(error = ?error, "image preference could not be read");
            Preference::Unreadable("Cannot read image preference".to_owned())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stored_autoload() -> Preference {
    Preference::Autoload
}

#[cfg(target_arch = "wasm32")]
pub fn remember_autoload(enabled: bool) -> Result<(), Said> {
    let storage = match preference_storage() {
        Kept::Storage(storage) => storage,
        Kept::Unavailable(reason) => return Err(Said::Something(reason)),
    };
    let value = match enabled {
        true => "false",
        false => "true",
    };
    match storage.set_item(IMAGE_PREFERENCE_KEY, value) {
        Ok(()) => Ok(()),
        Err(error) => {
            warn!(error = ?error, "image preference could not be saved");
            Err(Said::Something("Cannot save image preference".to_owned()))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remember_autoload(_enabled: bool) -> Result<(), Said> {
    Ok(())
}

#[component]
pub fn Markdown(text: String) -> impl IntoView {
    let mut count = 0;
    view! { <div class="post-text markdown-content">
        {segments(&text).into_iter().map(|segment| match segment {
            Segment::Text(text) => view! { <span>{text}</span> }.into_any(),
            Segment::Image { url, alt } => {
                count += 1;
                match count > MAX_IMAGES_PER_POST {
                    true => view! { <span class="media-placeholder">{format!("Image: {alt} — display limit: 2 images per post")}</span> }.into_any(),
                    false => view! { <ExternalImage url=url alt=alt /> }.into_any(),
                }
            }
        }).collect_view()}
    </div> }
}

enum Reachable {
    Fetchable,
    Refused(String),
}

fn reachability(url: &str) -> Reachable {
    let platform = use_platform_bridge();
    match platform.can_autoload_images() {
        true => match image_url(url) {
            Ok(_checked) => Reachable::Fetchable,
            Err(reason) => {
                warn!(reason = %reason, "image link refused before any request");
                Reachable::Refused(reason)
            }
        },
        false => Reachable::Refused(browser_image_error(url, secure_origin().https())),
    }
}

pub enum Origin {
    Https,
    Plain,
}

impl Origin {
    fn https(&self) -> bool {
        match self {
            Origin::Https => true,
            Origin::Plain => false,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn secure_origin() -> Origin {
    let Some(window) = web_sys::window() else {
        return Origin::Plain;
    };
    match window.location().protocol() {
        Ok(protocol) => match protocol.as_str() {
            "https:" => Origin::Https,
            _ => Origin::Plain,
        },
        Err(error) => {
            warn!(error = ?error, "page protocol could not be read");
            Origin::Plain
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn secure_origin() -> Origin {
    Origin::Plain
}

#[derive(Clone, Copy)]
struct Showing {
    data: RwSignal<Option<FetchedImage>>,
    message: RwSignal<Said>,
    loading: RwSignal<bool>,
    requested: RwSignal<bool>,
    attempted: RwSignal<bool>,
    generation: RwSignal<u64>,
}

impl Showing {
    fn starting(refusal: Said) -> Self {
        Self {
            data: RwSignal::new(None),
            message: RwSignal::new(refusal),
            loading: RwSignal::new(false),
            requested: RwSignal::new(false),
            attempted: RwSignal::new(false),
            generation: RwSignal::new(0),
        }
    }

    fn forget(&self) {
        self.data.set(None);
        self.requested.set(false);
        self.attempted.set(true);
        self.generation.update(|value| *value += 1);
    }

    fn wanted(&self, state: AppState) -> bool {
        self.requested.get() || (state.autoload_images.get() && !self.attempted.get())
    }

    fn begin(&self) {
        self.loading.set(true);
        self.attempted.set(true);
        self.requested.set(false);
        self.message.set(Said::Nothing);
    }
}

fn fetch_image(state: AppState, showing: Showing, url: StoredValue<String>, permitted: bool) {
    if !permitted
        || !showing.wanted(state)
        || showing.loading.get()
        || state.media_inflight.get() >= CONCURRENT_IMAGE_FETCHES
    {
        return;
    }
    if Budget::of(state.media_bytes.get_untracked()).full() {
        showing.message.set(Said::Something(
            "Displayed media reached 64 MiB. Clear media in Settings to load more.".to_owned(),
        ));
        showing.requested.set(false);
        showing.attempted.set(true);
        return;
    }
    showing.begin();
    let epoch = state.media_epoch.get_untracked();
    let request_generation = showing.generation.get_untracked();
    state.media_inflight.update(|count| *count += 1);
    let url = url.get_value();
    leptos::task::spawn_local(async move {
        let result = media::external_image(url).await;
        state.media_inflight.update(released);
        showing.loading.set(false);
        if showing.generation.get_untracked() != request_generation
            || state.media_epoch.get_untracked() != epoch
        {
            return;
        }
        match result {
            Ok(image) => keep(state, image, showing.data, showing.message),
            Err(failure) => {
                warn!(failure = %failure, "external image could not be fetched");
                showing.message.set(Said::Something(failure.reported()));
            }
        }
    });
}

#[component]
fn ExternalImage(url: String, alt: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let reachable = reachability(&url);
    let permitted = matches!(reachable, Reachable::Fetchable);
    let showing = Showing::starting(match reachable {
        Reachable::Fetchable => Said::Nothing,
        Reachable::Refused(reason) => Said::Something(reason),
    });
    let data = showing.data;
    let message = showing.message;
    let loading = showing.loading;
    let mounted_epoch = state.media_epoch.get_untracked();
    let url = StoredValue::new(url);
    Effect::new(move |_ran_before: Option<()>| {
        if state.media_epoch.get() != mounted_epoch {
            showing.forget();
        }
    });
    Effect::new(move |_ran_before: Option<()>| {
        fetch_image(state, showing, url, permitted);
    });
    let load = move |_clicked: leptos::ev::MouseEvent| {
        showing.requested.set(true);
    };
    on_cleanup(move || {
        let held = data.get_untracked();
        let Some(previous) = held else {
            return;
        };
        state.media_bytes.update(|bytes| {
            *bytes = Budget::of(*bytes)
                .releasing(previous.display_bytes())
                .spent()
        });
    });
    let placeholder_alt = alt.clone();
    let offers_load = move || match message.get() {
        Said::Something(_) => permitted && !loading.get(),
        Said::Nothing => permitted && !loading.get() && !state.autoload_images.get(),
    };
    let retrying = move || matches!(message.get(), Said::Something(_));
    view! {
        <span class="external-image">
            {move || match data.get() {
                Some(image) => view! { <img src=image.data_url alt=alt.clone() /> }.into_any(),
                None => view! {
                    <span class="media-placeholder">
                        <span>{placeholder_alt.clone()}</span>
                        {move || match loading.get() { true => "Loading checked image…", false => "External image" }}
                        {move || match message.get() {
                            Said::Something(message) => view! { <span>{message}</span> }.into_any(),
                            Said::Nothing => ().into_any(),
                        }}
                        {move || match offers_load() {
                            true => view! {
                                <button class="btn btn-secondary btn-sm" on:click=load>
                                    {move || match retrying() { true => "Retry image", false => "Load image" }}
                                </button>
                            }.into_any(),
                            false => ().into_any(),
                        }}
                    </span>
                }.into_any(),
            }}
        </span>
    }
}

fn released(count: &mut usize) {
    let Some(left) = count.checked_sub(1) else {
        return;
    };
    *count = left;
}

fn keep(
    state: AppState,
    image: FetchedImage,
    data: RwSignal<Option<FetchedImage>>,
    message: RwSignal<Said>,
) {
    let held = Budget::of(state.media_bytes.get_untracked());
    match held.holding(image.display_bytes()) {
        Room::Fits(total) => {
            state.media_bytes.set(total.spent());
            data.set(Some(image));
        }
        Room::Exhausted => message.set(Said::Something(
            "Image exceeds remaining display budget. Clear media in Settings.".to_owned(),
        )),
    }
}
