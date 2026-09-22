use leptos::prelude::*;
use wire_app_shared::{render_avatar_svg, render_identicon_svg, AuthorId};
use super::mock_data::{mock_author_auditor, mock_author_investigator, mock_hawk_avatar_bytes};

/// Variant A: Baseline V0 Avatar & Identicon
/// Shows the standard pixel-art avatar rendering and identicon fallback across various sizes.
#[component]
pub fn IdentityVariantA() -> impl IntoView {
    let (investigator, _, _) = mock_author_investigator();
    let hawk_pixels = mock_hawk_avatar_bytes();
    let avatar_svg_48 = render_avatar_svg(&hawk_pixels, 48);
    let avatar_svg_36 = render_avatar_svg(&hawk_pixels, 36);
    let identicon_svg_48 = render_identicon_svg(&investigator, 48);
    let identicon_svg_36 = render_identicon_svg(&investigator, 36);

    view! {
        <div class="identity-baseline-row">
            <div class="avatar-sample-item">
                <div class="avatar-container" style="width: 48px; height: 48px;" inner_html=avatar_svg_48 />
                <div class="sample-label">
                    <strong>"Pixel Avatar (48px)"</strong>
                    <span>"512-byte user drawing"</span>
                </div>
            </div>
            <div class="avatar-sample-item">
                <div class="avatar-container" style="width: 36px; height: 36px;" inner_html=avatar_svg_36 />
                <div class="sample-label">
                    <strong>"Pixel Avatar (36px)"</strong>
                    <span>"Feed card default"</span>
                </div>
            </div>
            <div class="avatar-sample-item">
                <div class="avatar-container" style="width: 48px; height: 48px;" inner_html=identicon_svg_48 />
                <div class="sample-label">
                    <strong>"Identicon Fallback (48px)"</strong>
                    <span>"Key hash algorithmic"</span>
                </div>
            </div>
            <div class="avatar-sample-item">
                <div class="avatar-container" style="width: 36px; height: 36px;" inner_html=identicon_svg_36 />
                <div class="sample-label">
                    <strong>"Identicon Fallback (36px)"</strong>
                    <span>"Zero-setup profile"</span>
                </div>
            </div>
        </div>
    }
}

/// Variant B: Cryptographic Identity Pill / Chip
/// Compact, interactive badge combining avatar, pseudonym, fingerprint, and verification indicator.
#[component]
pub fn IdentityVariantB() -> impl IntoView {
    let (author, name, handle) = mock_author_investigator();
    let hawk_pixels = mock_hawk_avatar_bytes();
    let avatar_svg = render_avatar_svg(&hawk_pixels, 28);
    let copied = RwSignal::new(false);

    let copy_key = move |_| {
        copied.set(true);
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            copied.set(false);
        });
    };

    view! {
        <div class="identity-pill-showcase">
            <div class="identity-chip" on:click=copy_key title="Click to copy BIP-340 public key">
                <div class="avatar-container" style="width: 28px; height: 28px;" inner_html=avatar_svg />
                <div class="chip-info">
                    <span class="chip-name">{name}</span>
                    <span class="chip-handle">{handle}</span>
                </div>
                <div class="chip-crypto">
                    <span class="chip-fp">"🔑 " {author.fingerprint()}</span>
                    <span class="chip-status-dot" title="Session unlocked & signing enabled"></span>
                </div>
                <span class="chip-copy-hint">
                    {move || if copied.get() { "✓ Copied" } else { "📋" }}
                </span>
            </div>
        </div>
    }
}

/// Variant C: Forensic Investigator Dossier Card
/// Full profile card detailing author provenance, verified public key, session security, and stats.
#[component]
pub fn IdentityVariantC() -> impl IntoView {
    let (author, name, handle) = mock_author_investigator();
    let hawk_pixels = mock_hawk_avatar_bytes();
    let avatar_svg = render_avatar_svg(&hawk_pixels, 64);
    let author_hex = author.to_hex();
    let copied = RwSignal::new(false);

    let copy_hex = move |_| {
        copied.set(true);
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            copied.set(false);
        });
    };

    view! {
        <div class="profile-card dossier-card">
            <div class="dossier-header">
                <div class="avatar-container dossier-avatar" style="width: 64px; height: 64px;" inner_html=avatar_svg />
                <div class="dossier-meta">
                    <div class="dossier-title-row">
                        <h3 class="dossier-name">{name}</h3>
                        <span class="badge badge-conf">"BIP-340 Schnorr"</span>
                        <span class="badge badge-block">"Session Active"</span>
                    </div>
                    <div class="dossier-handle-row">
                        <span class="dossier-handle">{handle}</span>
                        <span class="dossier-fingerprint">"Fingerprint: " {author.fingerprint()}</span>
                    </div>
                </div>
            </div>

            <div class="dossier-key-box">
                <div class="key-box-label">
                    <span>"BIP-340 x-only Public Key (32 bytes / 64 hex characters)"</span>
                    <button type="button" class="key-copy-btn" on:click=copy_hex>
                        {move || if copied.get() { "✓ Copied to clipboard" } else { "Copy Full Key" }}
                    </button>
                </div>
                <code class="key-hex-full">{author_hex}</code>
            </div>

            <div class="dossier-stats-grid">
                <div class="dossier-stat">
                    <span class="dossier-stat-lbl">"First Indexed Block"</span>
                    <span class="dossier-stat-val">"#4,892,600"</span>
                </div>
                <div class="dossier-stat">
                    <span class="dossier-stat-lbl">"Confirmed Dispatches"</span>
                    <span class="dossier-stat-val">"34 Records"</span>
                </div>
                <div class="dossier-stat">
                    <span class="dossier-stat-lbl">"Primary Wire"</span>
                    <span class="dossier-stat-val">"/rosint"</span>
                </div>
                <div class="dossier-stat">
                    <span class="dossier-stat-lbl">"Storage State"</span>
                    <span class="dossier-stat-val">"IndexedDB Isolated"</span>
                </div>
            </div>

            <div class="dossier-actions">
                <button type="button" class="btn btn-secondary btn-sm">"Export Encrypted Vault Backup"</button>
                <button type="button" class="btn btn-warning btn-sm">"Explicit Session Lock"</button>
            </div>
        </div>
    }
}

/// Variant D: Pixel-Art Avatar Matrix & Gallery
/// Shows how the 16-color palette renders different badges, emblems and identicons.
#[component]
pub fn IdentityVariantD() -> impl IntoView {
    let hawk = render_avatar_svg(&mock_hawk_avatar_bytes(), 40);
    let (a1, _, _) = mock_author_investigator();
    let (a2, _, _) = mock_author_auditor();
    let (_a3, _, _) = mock_author_investigator();

    let id1 = render_identicon_svg(&a1, 40);
    let id2 = render_identicon_svg(&a2, 40);
    let id3 = render_identicon_svg(&AuthorId([0x55; 32]), 40);

    view! {
        <div class="avatar-matrix-grid">
            <div class="matrix-card">
                <div class="avatar-container" style="width: 40px; height: 40px;" inner_html=hawk />
                <div class="matrix-info">
                    <strong>"Falcon Shield"</strong>
                    <span>"URMA 16-color pixel art"</span>
                </div>
            </div>
            <div class="matrix-card">
                <div class="avatar-container" style="width: 40px; height: 40px;" inner_html=id1 />
                <div class="matrix-info">
                    <strong>"Identicon #3B8D"</strong>
                    <span>"Alex Rivera key seed"</span>
                </div>
            </div>
            <div class="matrix-card">
                <div class="avatar-container" style="width: 40px; height: 40px;" inner_html=id2 />
                <div class="matrix-info">
                    <strong>"Identicon #7C1A"</strong>
                    <span>"Elena Vance key seed"</span>
                </div>
            </div>
            <div class="matrix-card">
                <div class="avatar-container" style="width: 40px; height: 40px;" inner_html=id3 />
                <div class="matrix-info">
                    <strong>"Identicon #5555"</strong>
                    <span>"Symmetric node hash"</span>
                </div>
            </div>
        </div>
    }
}

/// Variant E: Interactive Avatar Drawing Studio
/// Fully functional interactive 32x32 sandbox demonstrating brush types, flood fill,
/// color picker, and history undo/redo (Ctrl+Z / Ctrl+Y).
#[component]
pub fn IdentityVariantStudio() -> impl IntoView {
    use wire_app_shared::{avatar::encode_avatar_pixels, decode_avatar_pixels, AvatarPalette};
    use crate::components::profile_editor::{apply_brush, flood_fill, BrushShape, Tool};

    let initial_pixels = decode_avatar_pixels(&mock_hawk_avatar_bytes());
    let pixels = RwSignal::new(initial_pixels);
    let color = RwSignal::new(14u8); // default yellow
    let tool = RwSignal::new(Tool::Pencil);
    let brush = RwSignal::new(BrushShape::Single);

    let history = RwSignal::new(Vec::<[u8; 1024]>::new());
    let future = RwSignal::new(Vec::<[u8; 1024]>::new());

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

    let can_undo = move || !history.with(|h| h.is_empty());
    let can_redo = move || !future.with(|f| f.is_empty());

    let fill_canvas = move |fill_color: u8| {
        push_undo(pixels.get_untracked());
        pixels.set([fill_color; 1024]);
    };

    view! {
        <div class="avatar-studio-showcase">
            <div class="avatar-studio-controls">
                <div class="control-section">
                    <span class="control-label">"Tool"</span>
                    <div class="tool-btn-group">
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Pencil { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Pencil)
                        >
                            "✏️ Pencil"
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Bucket { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Bucket)
                        >
                            "🪣 Bucket"
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Eraser { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Eraser)
                        >
                            "🧹 Eraser"
                        </button>
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Eyedropper { "avatar-tool-btn active" } else { "avatar-tool-btn" }
                            on:click=move |_| tool.set(Tool::Eyedropper)
                        >
                            "💧 Picker"
                        </button>
                    </div>
                </div>

                <div class="control-section">
                    <span class="control-label">"Brush Shape"</span>
                    <div class="brush-btn-group">
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Single { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Single)
                        >
                            <span class="brush-preview-dot dot-1"></span>
                            <span>"1px"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Square2 { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Square2)
                        >
                            <span class="brush-preview-dot dot-2"></span>
                            <span>"2×2"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Square3 { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Square3)
                        >
                            <span class="brush-preview-dot dot-3"></span>
                            <span>"3×3"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if brush.get() == BrushShape::Cross { "avatar-brush-btn active" } else { "avatar-brush-btn" }
                            on:click=move |_| brush.set(BrushShape::Cross)
                        >
                            <span class="brush-preview-cross">"+"</span>
                            <span>"Cross"</span>
                        </button>
                    </div>
                </div>

                <div class="control-section">
                    <span class="control-label">"History Actions"</span>
                    <div class="history-btn-group">
                        <button
                            type="button"
                            class="avatar-action-btn"
                            disabled=move || !can_undo()
                            on:click=move |_| undo()
                        >
                            <span>"↶ Undo"</span>
                            <small>"Ctrl+Z"</small>
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            disabled=move || !can_redo()
                            on:click=move |_| redo()
                        >
                            <span>"↷ Redo"</span>
                            <small>"Ctrl+Y"</small>
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            on:click=move |_| fill_canvas(color.get_untracked())
                        >
                            "🎨 Fill"
                        </button>
                        <button
                            type="button"
                            class="avatar-action-btn"
                            on:click=move |_| fill_canvas(15)
                        >
                            "🗑️ Clear"
                        </button>
                    </div>
                </div>
            </div>

            <div class="palette-container">
                <div class="palette-meta">
                    <span class="control-label">"URMA 16-Color Palette"</span>
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
                        <span>"32 × 32 Canvas"</span>
                        <span>"Try brushes, bucket, or undo"</span>
                    </div>
                </div>

                <div class="avatar-preview-column">
                    <div class="preview-card-section">
                        <span class="preview-title">"Scaled Preview (96px)"</span>
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
        </div>
    }
}
