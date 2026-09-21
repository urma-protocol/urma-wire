use crate::{platform::tauri_call, state::AppState};
use leptos::prelude::*;
use serde_json::json;
use wire_app_shared::{
    avatar::encode_avatar_pixels, decode_avatar_pixels, live_model::Publication, render_avatar_svg,
    AvatarPalette,
};

#[component]
pub fn ProfileEditor() -> impl IntoView {
    let state = expect_context::<AppState>();
    let nickname = RwSignal::new(String::new());
    let pixels = RwSignal::new([15u8; 1024]);
    let color = RwSignal::new(0u8);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
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
        if let Some(author) = state.author_id() {
            if let Some(identity) = state
                .public_identities
                .get_untracked()
                .get(&author.to_hex())
            {
                nickname.set(identity.nickname.clone().unwrap_or_default());
                pixels.set(
                    identity
                        .avatar
                        .as_ref()
                        .and_then(|bytes| bytes.as_slice().try_into().ok())
                        .map(decode_avatar_pixels)
                        .unwrap_or([15; 1024]),
                );
            }
        }
    };
    let disabled = move || busy.get() || state.following.get() || state.author_id().is_none();
    view! {
        <div class="settings-group">
            <h3 class="group-title">"Public profile"</h3>
            <p>"Your nickname and drawing are public, separate transactions signed by your active identity. Names are not unique. Confirmed updates appear after indexing."</p>
            <button class="btn btn-secondary" disabled=disabled on:click=load_confirmed>"Load confirmed profile"</button>
            <label>"Nickname"<textarea class="input" rows="2" prop:value=move || nickname.get() on:input=move |event| nickname.set(event_target_value(&event)) /></label>
            <p class="action-desc">{move || nickname.read().len()} " / 32760 UTF-8 bytes. Empty uses your key fingerprint. Whitespace and Unicode are preserved exactly."</p>
            <button class="btn btn-primary" disabled=move || { disabled() || nickname.read().len() > 32760 }
                on:click=move |_| prepare("profile")>"Review nickname & fee"</button>
            <h3 class="group-title">"Draw your avatar"</h3>
            <p>"Select a color, then click or tap pixels on the 32 × 32 canvas. Mouse dragging also draws. Each pixel is one of these 16 colors."</p>
            <div class="avatar-palette" role="group" aria-label="Avatar colors">
                {(0u8..16).map(|index| view! {
                    <button type="button" class="avatar-swatch" style=format!("background:{}", AvatarPalette::hex(index))
                        aria-label=format!("Color {index:X}: {}", AvatarPalette::hex(index)) aria-pressed=move || color.get() == index
                        on:click=move |_| color.set(index) />
                }).collect_view()}
            </div>
            <div class="avatar-editor-layout">
                <div class="avatar-canvas" role="group" aria-label="32 by 32 avatar drawing canvas">
                    {(0usize..1024).map(|index| view! {
                        <button type="button" class="avatar-pixel" aria-label=format!("Pixel {}, {}", index % 32 + 1, index / 32 + 1)
                            style=move || format!("background:{}", AvatarPalette::hex(pixels.with(|pixels| pixels[index])))
                            on:pointerdown=move |event| { event.prevent_default(); pixels.update(|pixels| pixels[index] = color.get_untracked()); }
                            on:pointerenter=move |event| { if event.buttons() & 1 != 0 { pixels.update(|pixels| pixels[index] = color.get_untracked()); } }
                            on:click=move |_| pixels.update(|pixels| pixels[index] = color.get_untracked()) />
                    }).collect_view()}
                </div>
                <div><p>"Preview"</p><div inner_html=move || render_avatar_svg(&encode_avatar_pixels(&pixels.get()), 96) /></div>
            </div>
            <button class="btn btn-secondary" on:click=move |_| pixels.set([color.get_untracked(); 1024])>"Fill with selected color"</button>
            <p class="action-desc">"512 pixel bytes · 520-byte URMA record (transaction overhead extra). No uploaded image or metadata."</p>
            <button class="btn btn-primary" disabled=disabled on:click=move |_| prepare("avatar")>"Review avatar & fee"</button>
            {move || error.get().map(|message| view! { <div class="alert alert-error">{message}</div> })}
        </div>
        <super::compose_box::PublicationPanel />
    }
}
