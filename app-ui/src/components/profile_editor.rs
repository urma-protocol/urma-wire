use crate::state::session::signing_author;
use crate::state::session::Signer;
use crate::{platform::tauri_call, state::AppState};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{
    avatar::encode_avatar_pixels, decode_avatar_pixels, live_model::Publication, render_avatar_svg,
    AvatarPalette,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Eraser,
    Bucket,
    Eyedropper,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrushShape {
    Single,
    Square2,
    Square3,
    Cross,
}

pub fn apply_brush(
    pixels: &mut [u8; 1024],
    cx: usize,
    cy: usize,
    brush: BrushShape,
    color_val: u8,
) {
    let mut paint = |x: i32, y: i32| {
        if (0..32).contains(&x) && (0..32).contains(&y) {
            pixels[(y as usize) * 32 + (x as usize)] = color_val;
        }
    };

    let x = cx as i32;
    let y = cy as i32;

    match brush {
        BrushShape::Single => {
            paint(x, y);
        }
        BrushShape::Square2 => {
            paint(x, y);
            paint(x + 1, y);
            paint(x, y + 1);
            paint(x + 1, y + 1);
        }
        BrushShape::Square3 => {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    paint(x + dx, y + dy);
                }
            }
        }
        BrushShape::Cross => {
            paint(x, y);
            paint(x - 1, y);
            paint(x + 1, y);
            paint(x, y - 1);
            paint(x, y + 1);
        }
    }
}

pub fn flood_fill(pixels: &mut [u8; 1024], start_x: usize, start_y: usize, fill_color: u8) {
    let target_color = pixels[start_y * 32 + start_x];
    if target_color == fill_color {
        return;
    }

    let mut queue = std::collections::VecDeque::new();
    let mut visited = [false; 1024];

    queue.push_back((start_x as i32, start_y as i32));
    visited[start_y * 32 + start_x] = true;

    while let Some((x, y)) = queue.pop_front() {
        let idx = (y as usize) * 32 + (x as usize);
        pixels[idx] = fill_color;

        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            if (0..32).contains(&nx) && (0..32).contains(&ny) {
                let nidx = (ny as usize) * 32 + (nx as usize);
                if !visited[nidx] && pixels[nidx] == target_color {
                    visited[nidx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
}

#[component]
pub fn ProfileEditor() -> impl IntoView {
    let state = expect_context::<AppState>();
    let nickname = RwSignal::new(String::new());
    let pixels = RwSignal::new([15u8; 1024]);
    let color = RwSignal::new(0u8);
    let tool = RwSignal::new(Tool::Pencil);
    let brush = RwSignal::new(BrushShape::Single);

    let history = RwSignal::new(Vec::<[u8; 1024]>::new());
    let future = RwSignal::new(Vec::<[u8; 1024]>::new());

    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let push_undo = move |prev: [u8; 1024]| {
        history.update(|h| {
            if h.len() >= 60 {
                h.remove(0);
            }
            h.push(prev);
        });
        future.set(Vec::new());
    };

    let undo = move || {
        history.update(|h| {
            if let Some(prev) = h.pop() {
                let current = pixels.get_untracked();
                future.update(|f| f.push(current));
                pixels.set(prev);
            }
        });
    };

    let redo = move || {
        future.update(|f| {
            if let Some(next) = f.pop() {
                let current = pixels.get_untracked();
                history.update(|h| h.push(current));
                pixels.set(next);
            }
        });
    };

    // Global keyboard shortcuts: Ctrl+Z (Undo), Ctrl+Y / Ctrl+Shift+Z (Redo)
    window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        // If active focus is inside a text input or textarea, leave default browser undo/redo intact
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(active) = doc.active_element() {
                let tag = active.tag_name().to_lowercase();
                if tag == "input" || tag == "textarea" {
                    return;
                }
            }
        }

        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        let key = ev.key();

        if is_ctrl {
            if key == "z" || key == "Z" {
                ev.prevent_default();
                if ev.shift_key() {
                    redo();
                } else {
                    undo();
                }
            } else if key == "y" || key == "Y" {
                ev.prevent_default();
                redo();
            }
        }
    });

    let fill_canvas = move |fill_color: u8| {
        push_undo(pixels.get_untracked());
        pixels.set([fill_color; 1024]);
    };

    let clear_canvas = move |_| {
        fill_canvas(15); // 15 is white in AvatarPalette
    };

    let prepare = move |kind: &'static str| {
        if busy.get_untracked() || state.following.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        let text = if kind == "profile" {
            nickname.get_untracked()
        } else {
            String::new()
        };
        let avatar =
            (kind == "avatar").then(|| encode_avatar_pixels(&pixels.get_untracked()).to_vec());
        let chain = state.active_chain.get_untracked();
        leptos::task::spawn_local(async move {
            match tauri_call::<Publication>(
                "quote_publication",
                &json!({
                    "chain": chain.code(), "kind": kind, "text": text, "avatar": avatar,
                    "target": null, "wire": "", "hashtags": []
                }),
            )
            .await
            {
                Ok(quote) => state.publication.set(Some(quote)),
                Err(message) => error.set(Some(message)),
            }
            busy.set(false);
        });
    };

    let load_confirmed = move |_| {
        let Signer::Author(author) = signing_author(state) else {
            return;
        };
        let identities = state.public_identities.get_untracked();
        let Some(identity) = identities.get(&author.to_hex()) else {
            return;
        };
        nickname.set(identity.nickname.clone().unwrap_or_default());
        let loaded_pixels = identity
            .avatar
            .as_ref()
            .and_then(|bytes| bytes.as_slice().try_into().ok())
            .map(decode_avatar_pixels)
            .unwrap_or([15; 1024]);
        push_undo(pixels.get_untracked());
        pixels.set(loaded_pixels);
    };

    let disabled = move || {
        busy.get() || state.following.get() || matches!(signing_author(state), Signer::Locked)
    };
    let can_undo = move || !history.with(|h| h.is_empty());
    let can_redo = move || !future.with(|f| f.is_empty());

    view! {
        <div class="settings-group">
            <h3 class="group-title">"Public profile"</h3>
            <p>"Your nickname and drawing are public, separate transactions signed by your active identity. Names are not unique. Confirmed updates appear after indexing."</p>
            <button class="btn btn-secondary" disabled=disabled on:click=load_confirmed>"Load confirmed profile"</button>
            <label>"Nickname"
                <textarea class="input" rows="2" prop:value=move || nickname.get() on:input=move |event| nickname.set(event_target_value(&event)) />
            </label>
            <p class="action-desc">{move || nickname.read().len()} " / 32760 UTF-8 bytes. Empty uses your key fingerprint. Whitespace and Unicode are preserved exactly."</p>
            <button class="btn btn-primary" disabled=move || { disabled() || nickname.read().len() > 32760 }
                on:click=move |_| prepare("profile")>"Review nickname & fee"</button>

            <h3 class="group-title">"Draw your avatar"</h3>
            <p>"Create your 32 × 32 pixel identity. Choose tools, brush sizes, and 16 URMA colors. Full keyboard undo (Ctrl+Z) and redo (Ctrl+Y / Ctrl+Shift+Z) supported."</p>

            // Drawing Toolbar: Tools, Brushes, and History Controls
            <div class="avatar-studio-controls">
                // Tool Selection
                <div class="control-section">
                    <span class="control-label">"Tool"</span>
                    <div class="tool-btn-group" role="group" aria-label="Drawing tools">
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Pencil { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Pencil)
                            title="Pencil / Paintbrush"
                        >
                            <span class="tool-icon">"✏️"</span>
                            <span class="tool-name">"Pencil"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Bucket { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Bucket)
                            title="Flood Fill Bucket"
                        >
                            <span class="tool-icon">"🪣"</span>
                            <span class="tool-name">"Bucket"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Eraser { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Eraser)
                            title="Eraser (paint background white)"
                        >
                            <span class="tool-icon">"🧹"</span>
                            <span class="tool-name">"Eraser"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Eyedropper { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Eyedropper)
                            title="Eyedropper / Color Picker"
                        >
                            <span class="tool-icon">"💧"</span>
                            <span class="tool-name">"Picker"</span>
                        </button>
                    </div>
                </div>

                // Brush Shapes & Sizes
                <div class="control-section">
                    <span class="control-label">"Brush Shape"</span>
                    <div class="brush-btn-group" role="group" aria-label="Brush sizes">
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Single { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Single)
                            title="1×1 Single Pixel"
                        >
                            <span class="brush-preview-dot dot-1"></span>
                            <span>"1px"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Square2 { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Square2)
                            title="2×2 Block Brush"
                        >
                            <span class="brush-preview-dot dot-2"></span>
                            <span>"2×2"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Square3 { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Square3)
                            title="3×3 Wide Brush"
                        >
                            <span class="brush-preview-dot dot-3"></span>
                            <span>"3×3"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Cross { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Cross)
                            title="Cross Shape Brush"
                        >
                            <span class="brush-preview-cross">"+"</span>
                            <span>"Cross"</span>
                        </button>
                    </div>
                </div>

                // Undo, Redo, and Canvas Actions
                <div class="control-section">
                    <span class="control-label">"History & Canvas"</span>
                    <div class="history-btn-group">
                        <button
                            type="button"
                            class="avatar-action-btn"
                            disabled=move || !can_undo()
                            on:click=move |_| undo()
                            title="Undo last action (Ctrl+Z)"
                        >
                            <span>"↶ Undo"</span>
                            <small>"Ctrl+Z"</small>
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            disabled=move || !can_redo()
                            on:click=move |_| redo()
                            title="Redo action (Ctrl+Y / Ctrl+Shift+Z)"
                        >
                            <span>"↷ Redo"</span>
                            <small>"Ctrl+Y"</small>
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            on:click=move |_| fill_canvas(color.get_untracked())
                            title="Fill entire canvas with currently selected color"
                        >
                            <span>"🎨 Fill All"</span>
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            on:click=clear_canvas
                            title="Reset canvas to blank white"
                        >
                            <span>"🗑️ Clear"</span>
                        </button>
                    </div>
                </div>
            </div>

            // 16-Color Palette
            <div class="palette-container">
                <div class="palette-meta">
                    <span class="control-label">"16-Color Reference Palette"</span>
                    <span class="active-color-badge">
                        <span class="active-color-swatch" style=move || format!("background:{}", AvatarPalette::hex(color.get()))></span>
                        <code class="active-color-hex">{move || format!("Color {:X}: {}", color.get(), AvatarPalette::hex(color.get()))}</code>
                    </span>
                </div>
                <div class="avatar-palette" role="group" aria-label="Avatar colors">
                    {(0u8..16).map(|index| view! {
                        <button
                            type="button"
                            class="avatar-swatch"
                            style=format!("background:{}", AvatarPalette::hex(index))
                            aria-label=format!("Color {index:X}: {}", AvatarPalette::hex(index))
                            aria-pressed=move || color.get() == index
                            on:click=move |_| {
                                color.set(index);
                                if tool.get_untracked() == Tool::Eraser {
                                    tool.set(Tool::Pencil);
                                }
                            }
                        />
                    }).collect_view()}
                </div>
            </div>

            // Canvas & Multi-Size Preview Layout
            <div class="avatar-editor-layout">
                <div class="canvas-wrapper">
                    <div class="avatar-canvas" role="group" aria-label="32 by 32 avatar drawing canvas">
                        {(0usize..1024).map(|index| {
                            let x = index % 32;
                            let y = index / 32;
                            view! {
                                <button
                                    type="button"
                                    class="avatar-pixel"
                                    aria-label=format!("Pixel {}, {}", x + 1, y + 1)
                                    style=move || format!("background:{}", AvatarPalette::hex(pixels.with(|p| p[index])))
                                    on:pointerdown=move |event| {
                                        event.prevent_default();
                                        match tool.get_untracked() {
                                            Tool::Eyedropper => {
                                                let picked = pixels.with(|p| p[index]);
                                                color.set(picked);
                                                tool.set(Tool::Pencil);
                                            }
                                            Tool::Bucket => {
                                                push_undo(pixels.get_untracked());
                                                let c = color.get_untracked();
                                                pixels.update(|p| flood_fill(p, x, y, c));
                                            }
                                            Tool::Eraser => {
                                                push_undo(pixels.get_untracked());
                                                let b = brush.get_untracked();
                                                pixels.update(|p| apply_brush(p, x, y, b, 15));
                                            }
                                            Tool::Pencil => {
                                                push_undo(pixels.get_untracked());
                                                let c = color.get_untracked();
                                                let b = brush.get_untracked();
                                                pixels.update(|p| apply_brush(p, x, y, b, c));
                                            }
                                        }
                                    }
                                    on:pointerenter=move |event| {
                                        if event.buttons() & 1 != 0 {
                                            match tool.get_untracked() {
                                                Tool::Pencil => {
                                                    let c = color.get_untracked();
                                                    let b = brush.get_untracked();
                                                    pixels.update(|p| apply_brush(p, x, y, b, c));
                                                }
                                                Tool::Eraser => {
                                                    let b = brush.get_untracked();
                                                    pixels.update(|p| apply_brush(p, x, y, b, 15));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                />
                            }
                        }).collect_view()}
                    </div>
                    <div class="canvas-coords-hint">
                        <span>"32 × 32 Grid"</span>
                        <span>"Click or drag to paint"</span>
                    </div>
                </div>

                // Multi-Scale Preview Gallery
                <div class="avatar-preview-column">
                    <div class="preview-card-section">
                        <span class="preview-title">"Preview: Large (96px)"</span>
                        <div class="avatar-preview-frame large" inner_html=move || render_avatar_svg(&encode_avatar_pixels(&pixels.get()), 96) />
                    </div>

                    <div class="preview-scales-row">
                        <div class="preview-card-section mini">
                            <span class="preview-title">"Feed (36px)"</span>
                            <div class="avatar-preview-frame" inner_html=move || render_avatar_svg(&encode_avatar_pixels(&pixels.get()), 36) />
                        </div>
                        <div class="preview-card-section mini">
                            <span class="preview-title">"Reply (24px)"</span>
                            <div class="avatar-preview-frame" inner_html=move || render_avatar_svg(&encode_avatar_pixels(&pixels.get()), 24) />
                        </div>
                    </div>
                </div>
            </div>

            <p class="action-desc">"512 pixel bytes · 520-byte URMA record (transaction overhead extra). Pixel data is encoded high-nibble first and signed atomically on-chain."</p>
            <button class="btn btn-primary" disabled=disabled on:click=move |_| prepare("avatar")>"Review avatar & fee"</button>
            {move || error.get().map(|message| view! { <div class="alert alert-error">{message}</div> })}
        </div>
        <super::compose_box::PublicationPanel />
    }
}
