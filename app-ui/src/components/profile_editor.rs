use crate::components::Actions;
use crate::components::PublicationPanel;
use crate::components::Warning;
use crate::config::avatar_when_unset;
use crate::config::nickname_when_unset;
use crate::config::AVATAR_BACKGROUND;
use crate::config::AVATAR_PREVIEW_PIXELS;
use crate::config::CARD_AVATAR_PIXELS;
use crate::config::MAX_NICKNAME_BYTES;
use crate::config::PROFILE_AVATAR_PIXELS;
use crate::data::publication;
use crate::state::session::signing_author;
use crate::state::session::Signer;
use crate::state::AppState;
use crate::state::Said;
use leptos::prelude::expect_context;
use leptos::prelude::window_event_listener;
use leptos::prelude::AriaAttributes;
use leptos::prelude::Callable;
use leptos::prelude::Callback;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::CustomAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::InnerHtmlAttribute;
use leptos::prelude::IntoView;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::Read;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Signal;
use leptos::prelude::StyleAttribute;
use leptos::prelude::Update;
use leptos::prelude::With;
use leptos::prelude::WithUntracked;
use leptos::prelude::{component, event_target_value, view};
use std::collections::VecDeque;
use tracing::warn;
use wire_app_core::avatar::Urma;
use wire_app_core::avatar::{
    decode_avatar_pixels, encode_avatar_pixels, render_avatar_svg, AvatarPalette,
};
use wire_app_core::feed::PublicIdentity;
use wire_app_core::publication::Content;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Bucket,
    Eraser,
    Eyedropper,
}

impl Tool {
    const ALL: [Tool; 4] = [Tool::Pencil, Tool::Bucket, Tool::Eraser, Tool::Eyedropper];

    fn name(self) -> &'static str {
        match self {
            Tool::Pencil => "Pencil",
            Tool::Bucket => "Bucket",
            Tool::Eraser => "Eraser",
            Tool::Eyedropper => "Picker",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Tool::Pencil => "Pencil: paint with the selected color",
            Tool::Bucket => "Bucket: flood-fill a connected area",
            Tool::Eraser => "Eraser: paint the white background",
            Tool::Eyedropper => "Picker: take a color from the canvas",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Tool::Pencil => "m15 4 5 5-11 11H4v-5ZM12 7l5 5",
            Tool::Bucket => {
                "M3 11l8-8 8 8-8 8ZM3 11h16M20 14c1.3 1.7 2 3 2 4a2 2 0 0 1-4 0c0-1 .7-2.3 2-4Z"
            }
            Tool::Eraser => "M13 4l7 7-8 8H8l-4-4ZM8 19h12M9 10l5 5",
            Tool::Eyedropper => "M3 21l1-4 9-9 3 3-9 9ZM15 5l4 4M17 3l4 4-3 3-4-4Z",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrushShape {
    Single,
    Square2,
    Square3,
    Cross,
}

impl BrushShape {
    const ALL: [BrushShape; 4] = [
        BrushShape::Single,
        BrushShape::Square2,
        BrushShape::Square3,
        BrushShape::Cross,
    ];

    fn label(self) -> &'static str {
        match self {
            BrushShape::Single => "1px",
            BrushShape::Square2 => "2×2",
            BrushShape::Square3 => "3×3",
            BrushShape::Cross => "Cross",
        }
    }

    fn title(self) -> &'static str {
        match self {
            BrushShape::Single => "Single pixel",
            BrushShape::Square2 => "2×2 block",
            BrushShape::Square3 => "3×3 block",
            BrushShape::Cross => "Cross shape",
        }
    }

    fn preview_class(self) -> &'static str {
        match self {
            BrushShape::Single => "brush-preview-dot dot-1",
            BrushShape::Square2 => "brush-preview-dot dot-2",
            BrushShape::Square3 => "brush-preview-dot dot-3",
            BrushShape::Cross => "brush-preview-cross",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            BrushShape::Cross => "+",
            BrushShape::Single => "",
            BrushShape::Square2 => "",
            BrushShape::Square3 => "",
        }
    }

    fn covers(self, x: usize, y: usize, px: usize, py: usize) -> bool {
        let near_x = px + 1 >= x && px <= x + 1;
        let near_y = py + 1 >= y && py <= y + 1;
        match self {
            BrushShape::Single => px == x && py == y,
            BrushShape::Square2 => px >= x && px <= x + 1 && py >= y && py <= y + 1,
            BrushShape::Square3 => near_x && near_y,
            BrushShape::Cross => (px == x && near_y) || (py == y && near_x),
        }
    }
}

pub fn apply_brush(
    pixels: &mut [u8; Urma::AVATAR_PIXELS],
    x: usize,
    y: usize,
    brush: BrushShape,
    color: u8,
) {
    for (index, pixel) in pixels.iter_mut().enumerate() {
        if brush.covers(x, y, index % Urma::AVATAR_SIDE, index / Urma::AVATAR_SIDE) {
            *pixel = color;
        }
    }
}

fn adjacent(index: usize) -> Vec<usize> {
    let x = index % Urma::AVATAR_SIDE;
    let y = index / Urma::AVATAR_SIDE;
    let mut cells = Vec::with_capacity(4);
    if x > 0 {
        cells.push(index - 1);
    }
    if x + 1 < Urma::AVATAR_SIDE {
        cells.push(index + 1);
    }
    if y > 0 {
        cells.push(index - Urma::AVATAR_SIDE);
    }
    if y + 1 < Urma::AVATAR_SIDE {
        cells.push(index + Urma::AVATAR_SIDE);
    }
    cells
}

pub fn flood_fill(pixels: &mut [u8; Urma::AVATAR_PIXELS], x: usize, y: usize, fill: u8) {
    let start = y * Urma::AVATAR_SIDE + x;
    let target = pixels[start];
    if target == fill {
        return;
    }
    let mut queue = VecDeque::from([start]);
    let mut visited = [false; Urma::AVATAR_PIXELS];
    visited[start] = true;
    loop {
        let Some(index) = queue.pop_front() else {
            break;
        };
        pixels[index] = fill;
        for next in adjacent(index) {
            if !visited[next] && pixels[next] == target {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Studio {
    pixels: RwSignal<[u8; Urma::AVATAR_PIXELS]>,
    color: RwSignal<u8>,
    tool: RwSignal<Tool>,
    brush: RwSignal<BrushShape>,
    history: RwSignal<Vec<[u8; Urma::AVATAR_PIXELS]>>,
    future: RwSignal<Vec<[u8; Urma::AVATAR_PIXELS]>>,
    notice: RwSignal<Said>,
}

impl Studio {
    const HISTORY_DEPTH: usize = 60;

    fn opened() -> Self {
        Self {
            pixels: RwSignal::new(avatar_when_unset()),
            color: RwSignal::new(0),
            tool: RwSignal::new(Tool::Pencil),
            brush: RwSignal::new(BrushShape::Single),
            history: RwSignal::new(Vec::new()),
            future: RwSignal::new(Vec::new()),
            notice: RwSignal::new(Said::Nothing),
        }
    }

    fn remember(&self) {
        let current = self.pixels.get_untracked();
        self.history.update(|held| {
            if held.len() >= Studio::HISTORY_DEPTH {
                held.remove(0);
            }
            held.push(current);
        });
        self.future.set(Vec::new());
    }

    fn undo(&self) {
        let mut held = self.history.get_untracked();
        let Some(previous) = held.pop() else {
            return;
        };
        self.history.set(held);
        let current = self.pixels.get_untracked();
        self.future.update(|next| next.push(current));
        self.pixels.set(previous);
    }

    fn redo(&self) {
        let mut held = self.future.get_untracked();
        let Some(next) = held.pop() else {
            return;
        };
        self.future.set(held);
        let current = self.pixels.get_untracked();
        self.history.update(|previous| previous.push(current));
        self.pixels.set(next);
    }

    fn fill(&self, color: u8) {
        self.remember();
        self.pixels.set([color; Urma::AVATAR_PIXELS]);
    }

    fn load(&self, pixels: [u8; Urma::AVATAR_PIXELS]) {
        self.remember();
        self.pixels.set(pixels);
    }

    fn stroke(&self, index: usize, color: u8) {
        let brush = self.brush.get_untracked();
        self.pixels.update(|pixels| {
            apply_brush(
                pixels,
                index % Urma::AVATAR_SIDE,
                index / Urma::AVATAR_SIDE,
                brush,
                color,
            )
        });
    }

    fn press(&self, index: usize) {
        match self.tool.get_untracked() {
            Tool::Eyedropper => {
                let picked = self.pixels.with_untracked(|pixels| pixels[index]);
                self.color.set(picked);
                self.tool.set(Tool::Pencil);
            }
            Tool::Bucket => {
                self.remember();
                let color = self.color.get_untracked();
                self.pixels.update(|pixels| {
                    flood_fill(
                        pixels,
                        index % Urma::AVATAR_SIDE,
                        index / Urma::AVATAR_SIDE,
                        color,
                    )
                });
            }
            Tool::Eraser => {
                self.remember();
                self.stroke(index, AVATAR_BACKGROUND);
            }
            Tool::Pencil => {
                self.remember();
                self.stroke(index, self.color.get_untracked());
            }
        }
    }

    fn drag(&self, index: usize) {
        match self.tool.get_untracked() {
            Tool::Pencil => self.stroke(index, self.color.get_untracked()),
            Tool::Eraser => self.stroke(index, AVATAR_BACKGROUND),
            Tool::Bucket => (),
            Tool::Eyedropper => (),
        }
    }

    fn choose(&self, color: u8) {
        self.color.set(color);
        if self.tool.get_untracked() == Tool::Eraser {
            self.tool.set(Tool::Pencil);
        }
    }

    fn shortcut(&self, event: &web_sys::KeyboardEvent) {
        if editing_text() {
            return;
        }
        match Shortcut::read(event) {
            Shortcut::Undo => {
                event.prevent_default();
                self.undo();
            }
            Shortcut::Redo => {
                event.prevent_default();
                self.redo();
            }
            Shortcut::Other => (),
        }
    }

    fn preview(&self, size: u32) -> String {
        editor_preview(&self.pixels.get(), size)
    }
}

pub fn editor_preview(pixels: &[u8; Urma::AVATAR_PIXELS], size: u32) -> String {
    match encode_avatar_pixels(pixels) {
        Ok(bytes) => render_avatar_svg(&bytes, size),
        Err(error) => {
            warn!(error = %error, "avatar preview refused invalid palette index");
            String::from("<span>Avatar preview unavailable: invalid palette index.</span>")
        }
    }
}

enum Shortcut {
    Undo,
    Redo,
    Other,
}

impl Shortcut {
    fn read(event: &web_sys::KeyboardEvent) -> Self {
        if !(event.ctrl_key() || event.meta_key()) {
            return Shortcut::Other;
        }
        match (event.key().as_str(), event.shift_key()) {
            ("z" | "Z", false) => Shortcut::Undo,
            ("z" | "Z", true) => Shortcut::Redo,
            ("y" | "Y", false) => Shortcut::Redo,
            ("y" | "Y", true) => Shortcut::Redo,
            _ => Shortcut::Other,
        }
    }
}

fn editing_text() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(document) = window.document() else {
        return false;
    };
    let Some(active) = document.active_element() else {
        return false;
    };
    let tag = active.tag_name().to_lowercase();
    tag == "input" || tag == "textarea"
}

#[derive(Clone, Copy)]
struct Review {
    state: AppState,
    busy: RwSignal<bool>,
    trouble: RwSignal<Said>,
}

impl Review {
    fn disabled(&self) -> bool {
        self.busy.get()
            || self.state.following.get()
            || matches!(signing_author(self.state), Signer::Locked)
    }

    fn quote(&self, content: Content) {
        if self.busy.get_untracked() || self.state.following.get_untracked() {
            return;
        }
        let review = *self;
        review.busy.set(true);
        review.trouble.set(Said::Nothing);
        let chain = review.state.active_chain.get_untracked();
        leptos::task::spawn_local(async move {
            match publication::quote(chain, content).await {
                Ok(quote) => review.state.publication.set(Some(quote)),
                Err(fault) => {
                    warn!(fault = %fault, "profile quote refused");
                    review.trouble.set(Said::Something(fault.message));
                }
            }
            review.busy.set(false);
        });
    }
}

fn load_confirmed(state: AppState, studio: Studio, nickname: RwSignal<String>) {
    let Signer::Author(author) = signing_author(state) else {
        return;
    };
    let identities = state.public_identities.get_untracked();
    let Some(identity) = identities.get(&author.to_hex()) else {
        return;
    };
    nickname.set(confirmed_nickname(identity.nickname.clone()));
    studio.load(confirmed_pixels(identity, studio.notice));
}

fn confirmed_nickname(nickname: Option<String>) -> String {
    let Some(name) = nickname else {
        return nickname_when_unset();
    };
    name
}

pub fn confirmed_pixels(
    identity: &PublicIdentity,
    notice: RwSignal<Said>,
) -> [u8; Urma::AVATAR_PIXELS] {
    notice.set(Said::Nothing);
    let Some(bytes) = identity.avatar.as_ref() else {
        return avatar_when_unset();
    };
    match <&[u8; Urma::AVATAR_BYTES]>::try_from(bytes.as_slice()) {
        Ok(packed) => decode_avatar_pixels(packed),
        Err(error) => {
            warn!(error = %error, "confirmed avatar is not 128 packed bytes; starting blank");
            notice.set(Said::Something(String::from(
                "Unsupported stored avatar. A blank canvas is shown; the stored bytes are unchanged.",
            )));
            avatar_when_unset()
        }
    }
}

#[component]
pub fn ProfileEditor() -> impl IntoView {
    let state = expect_context::<AppState>();
    let studio = Studio::opened();
    let review = Review {
        state,
        busy: RwSignal::new(false),
        trouble: RwSignal::new(Said::Nothing),
    };
    let nickname = RwSignal::new(String::new());
    window_event_listener(leptos::ev::keydown, move |event| studio.shortcut(&event));
    let disabled = Signal::derive(move || review.disabled());
    let quote = Callback::new(move |content: Content| review.quote(content));

    view! {
        <div class="settings-group">
            <h3 class="group-title">"Public profile"</h3>
            <p>"Your nickname and drawing are public, separate transactions signed by your active identity. Names are not unique. Confirmed updates appear after indexing."</p>
            <Actions>
                <button class="btn btn-secondary btn-sm" disabled=disabled
                    on:click=move |_clicked| load_confirmed(state, studio, nickname)>"Load confirmed profile"</button>
            </Actions>
            <NicknameEditor nickname=nickname disabled=disabled quote=quote />

            <h3 class="group-title">"Draw your avatar"</h3>
            <p>"Create your 16 × 16 pixel identity. Choose tools, brush sizes, and 16 URMA colors. Full keyboard undo (Ctrl+Z) and redo (Ctrl+Y / Ctrl+Shift+Z) supported."</p>
            <Warning message=studio.notice.into() />
            <Toolbar studio=studio />
            <Palette studio=studio />
            <div class="avatar-editor-layout">
                <Canvas studio=studio />
                <Previews studio=studio />
            </div>
            <p class="action-desc">"128 packed pixel bytes · 136-byte URMA record (transaction overhead extra). Pixel data is encoded high-nibble first and signed atomically on-chain."</p>
            <Actions>
                <button class="btn btn-primary" disabled=disabled
                    on:click=move |_clicked| match encode_avatar_pixels(&studio.pixels.get_untracked()) {
                        Ok(bytes) => quote.run(Content::Avatar(bytes.to_vec())),
                        Err(error) => {
                            warn!(error = %error, "avatar review refused invalid palette index");
                            review.trouble.set(Said::Something(error.to_string()));
                        }
                    }>
                    "Review avatar & fee"
                </button>
            </Actions>
            <Warning message=review.trouble.into() />
        </div>
        <PublicationPanel />
    }
}

#[component]
fn NicknameEditor(
    nickname: RwSignal<String>,
    disabled: Signal<bool>,
    quote: Callback<Content>,
) -> impl IntoView {
    let too_long = move || nickname.read().len() > MAX_NICKNAME_BYTES;
    view! {
        <label class="field">"Nickname"
            <textarea rows="2" prop:value=move || nickname.get()
                on:input=move |event| nickname.set(event_target_value(&event)) />
            <span class="field-hint">
                {move || nickname.read().len()} " / " {MAX_NICKNAME_BYTES}
                " UTF-8 bytes. Empty uses your key fingerprint. Whitespace and Unicode are preserved exactly."
            </span>
        </label>
        <Actions>
            <button class="btn btn-primary" disabled=move || disabled.get() || too_long()
                on:click=move |_clicked| quote.run(Content::Nickname(nickname.get_untracked()))>
                "Review nickname & fee"
            </button>
        </Actions>
    }
}

#[component]
fn Toolbar(studio: Studio) -> impl IntoView {
    let can_undo = move || studio.history.with(|held| !held.is_empty());
    let can_redo = move || studio.future.with(|held| !held.is_empty());
    view! {
        <div class="avatar-studio-controls">
            <div class="control-section">
                <span class="control-label">"Tool"</span>
                <div class="tool-btn-group" role="group" aria-label="Drawing tools">
                    {Tool::ALL.into_iter().map(|tool| view! { <ToolButton studio=studio tool=tool /> }).collect_view()}
                </div>
            </div>
            <div class="control-section">
                <span class="control-label">"Brush"</span>
                <div class="brush-btn-group" role="group" aria-label="Brush sizes">
                    {BrushShape::ALL.into_iter().map(|brush| view! { <BrushButton studio=studio brush=brush /> }).collect_view()}
                </div>
            </div>
            <div class="control-section">
                <span class="control-label">"History & canvas"</span>
                <div class="history-btn-group">
                    <button type="button" class="avatar-action-btn" title="Undo (Ctrl+Z)"
                        disabled=move || !can_undo() on:click=move |_clicked| studio.undo()>
                        <span>"↶ Undo"</span><small>"Ctrl+Z"</small>
                    </button>
                    <button type="button" class="avatar-action-btn" title="Redo (Ctrl+Y or Ctrl+Shift+Z)"
                        disabled=move || !can_redo() on:click=move |_clicked| studio.redo()>
                        <span>"↷ Redo"</span><small>"Ctrl+Y"</small>
                    </button>
                    <button type="button" class="avatar-action-btn" title="Fill the whole canvas with the selected color"
                        on:click=move |_clicked| studio.fill(studio.color.get_untracked())>
                        <span class="tool-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="2"/></svg></span>
                        <span>"Fill all"</span>
                    </button>
                    <button type="button" class="avatar-action-btn" title="Reset the canvas to white"
                        on:click=move |_clicked| studio.fill(AVATAR_BACKGROUND)>
                        <span class="tool-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/></svg></span>
                        <span>"Clear"</span>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ToolButton(studio: Studio, tool: Tool) -> impl IntoView {
    view! {
        <button type="button"
            class=move || match studio.tool.get() == tool { true => "avatar-tool-btn active", false => "avatar-tool-btn" }
            aria-pressed=move || (studio.tool.get() == tool).to_string()
            title=tool.title()
            on:click=move |_clicked| studio.tool.set(tool)>
            <span class="tool-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d=tool.icon()/></svg></span>
            <span class="tool-name">{tool.name()}</span>
        </button>
    }
}

#[component]
fn BrushButton(studio: Studio, brush: BrushShape) -> impl IntoView {
    view! {
        <button type="button"
            class=move || match studio.brush.get() == brush { true => "avatar-brush-btn active", false => "avatar-brush-btn" }
            aria-pressed=move || (studio.brush.get() == brush).to_string()
            title=brush.title()
            on:click=move |_clicked| studio.brush.set(brush)>
            <span class=brush.preview_class()>{brush.glyph()}</span>
            <span>{brush.label()}</span>
        </button>
    }
}

#[component]
fn Palette(studio: Studio) -> impl IntoView {
    let hex = move || AvatarPalette::hex(studio.color.get());
    view! {
        <div class="palette-container">
            <div class="palette-meta">
                <span class="control-label">"16-color reference palette"</span>
                <span class="active-color-badge">
                    <span class="active-color-swatch" style=move || format!("background:{}", hex())></span>
                    <code class="active-color-hex">{move || format!("Color {:X}: {}", studio.color.get(), hex())}</code>
                </span>
            </div>
            <div class="avatar-palette" role="group" aria-label="Avatar colors">
                {(0u8..16).map(|index| view! {
                    <button type="button" class="avatar-swatch"
                        style=format!("background:{}", AvatarPalette::hex(index))
                        aria-label=format!("Color {index:X}: {}", AvatarPalette::hex(index))
                        aria-pressed=move || (studio.color.get() == index).to_string()
                        on:click=move |_clicked| studio.choose(index) />
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn Canvas(studio: Studio) -> impl IntoView {
    view! {
        <AvatarGrid pixels=studio.pixels
            press=Callback::new(move |index| studio.press(index))
            drag=Callback::new(move |index| studio.drag(index)) />
    }
}

#[component]
pub fn AvatarGrid(
    pixels: RwSignal<[u8; Urma::AVATAR_PIXELS]>,
    press: Callback<usize>,
    drag: Callback<usize>,
) -> impl IntoView {
    view! {
        <div class="canvas-wrapper">
            <div class="avatar-canvas" role="group" aria-label="16 by 16 avatar drawing canvas"
                style=format!("--avatar-side:{}", Urma::AVATAR_SIDE)>
                {(0usize..Urma::AVATAR_PIXELS).map(|index| view! {
                    <button type="button" class="avatar-pixel"
                        aria-label=format!("Pixel {}, {}", index % Urma::AVATAR_SIDE + 1, index / Urma::AVATAR_SIDE + 1)
                        style=move || format!("background:{}", AvatarPalette::hex(pixels.with(|pixels| pixels[index])))
                        on:pointerdown=move |event| { event.prevent_default(); press.run(index); }
                        on:pointerenter=move |event| { if event.buttons() & 1 != 0 { drag.run(index); } } />
                }).collect_view()}
            </div>
            <div class="canvas-coords-hint">
                <span>"16 × 16 grid"</span>
                <span>"Click or drag to paint"</span>
            </div>
        </div>
    }
}

#[component]
fn Previews(studio: Studio) -> impl IntoView {
    view! {
        <div class="avatar-preview-column">
            <div class="preview-card-section">
                <span class="preview-title">{format!("Preview · {AVATAR_PREVIEW_PIXELS} px")}</span>
                <div class="avatar-preview-frame large" inner_html=move || studio.preview(AVATAR_PREVIEW_PIXELS) />
            </div>
            <div class="preview-scales-row">
                <div class="preview-card-section mini">
                    <span class="preview-title">{format!("Profile · {PROFILE_AVATAR_PIXELS} px")}</span>
                    <div class="avatar-preview-frame" inner_html=move || studio.preview(PROFILE_AVATAR_PIXELS) />
                </div>
                <div class="preview-card-section mini">
                    <span class="preview-title">{format!("Card · {CARD_AVATAR_PIXELS} px")}</span>
                    <div class="avatar-preview-frame" inner_html=move || studio.preview(CARD_AVATAR_PIXELS) />
                </div>
            </div>
        </div>
    }
}
