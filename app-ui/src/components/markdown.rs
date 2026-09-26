use crate::components::budget::Budget;
use crate::components::budget::Room;
use crate::components::FollowChip;
use crate::components::FollowToken;
use crate::config::CONCURRENT_IMAGE_FETCHES;
use crate::data::media;
use crate::platform::use_platform;
use crate::state::AppState;
use crate::state::Said;
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
use wire_app_core::body::{body, Piece};
use wire_app_core::config::MAX_IMAGES_PER_POST;
use wire_app_core::markdown::browser_image_error;
use wire_app_core::markdown::image_url;
use wire_app_core::markdown::FetchedImage;

#[component]
pub fn PostText(text: String, wire: String, hashtags: Vec<String>) -> impl IntoView {
    let read = body(&text, &wire, &hashtags);
    let mut count = 0;
    view! {
        <Unwritten labels=read.unwritten />
        <div class="post-text markdown-content">
            {read.pieces.into_iter().map(|piece| match piece {
                Piece::Text(text) => view! { <span>{text}</span> }.into_any(),
                Piece::Label { written, spelled } => view! { <FollowToken spelled=spelled written=written /> }.into_any(),
                Piece::Image { url, alt } => {
                    count += 1;
                    match count > MAX_IMAGES_PER_POST {
                        true => view! { <span class="media-placeholder">{format!("Image: {alt} — display limit: 2 images per post")}</span> }.into_any(),
                        false => view! { <ExternalImage url=url alt=alt /> }.into_any(),
                    }
                }
            }).collect_view()}
        </div>
    }
}

#[component]
fn Unwritten(labels: Vec<String>) -> impl IntoView {
    if labels.is_empty() {
        return ().into_any();
    }
    view! {
        <p class="topic-labels">
            {labels.into_iter().map(|label| view! { <FollowChip spelled=label /> }).collect_view()}
        </p>
    }
    .into_any()
}

enum Reachable {
    Fetchable,
    Refused(String),
}

fn reachability(url: &str) -> Reachable {
    match use_platform().can_autoload_images() {
        true => match image_url(url) {
            Ok(_checked) => Reachable::Fetchable,
            Err(reason) => {
                warn!(reason = %reason, "image link refused before any request");
                Reachable::Refused(reason.to_string())
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
            Err(fault) => {
                warn!(fault = %fault, "external image could not be fetched");
                showing.message.set(Said::Something(fault.message));
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
    on_cleanup(move || release(state, data.get_untracked()));
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

fn release(state: AppState, held: Option<FetchedImage>) {
    let Some(previous) = held else {
        return;
    };
    match previous.display_bytes() {
        Ok(bytes) => state
            .media_bytes
            .update(|spent| *spent = Budget::of(*spent).releasing(bytes).spent()),
        Err(error) => warn!(error = %error, "released image carries no display size"),
    }
}

fn keep(
    state: AppState,
    image: FetchedImage,
    data: RwSignal<Option<FetchedImage>>,
    message: RwSignal<Said>,
) {
    let held = Budget::of(state.media_bytes.get_untracked());
    let room = match image.display_bytes() {
        Ok(bytes) => held.holding(bytes),
        Err(error) => {
            warn!(error = %error, "fetched image size overflows the display budget");
            Room::Exhausted
        }
    };
    match room {
        Room::Fits(total) => {
            state.media_bytes.set(total.spent());
            data.set(Some(image));
        }
        Room::Exhausted => message.set(Said::Something(
            "Image exceeds remaining display budget. Clear media in Settings.".to_owned(),
        )),
    }
}
