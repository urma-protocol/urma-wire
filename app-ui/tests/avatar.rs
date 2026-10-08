use leptos::prelude::{view, Callback, Owner, RenderHtml, RwSignal, WithUntracked};
use std::collections::BTreeMap;
use wire_app_core::{
    avatar::{render_identicon_svg, Urma},
    feed::PublicIdentity,
    model::AuthorId,
};
use wire_app_ui::{
    components::{
        avatar_view::drawn,
        profile_editor::{
            apply_brush, confirmed_pixels, editor_preview, flood_fill, AvatarGrid, BrushShape,
        },
    },
    config::AVATAR_BACKGROUND,
    state::Said,
};

#[test]
fn editor_renders_256_cells_with_row_major_coordinates_and_16_css_columns() {
    let owner = Owner::new();
    let html = owner.with(|| {
        let pixels = RwSignal::new(std::array::from_fn(|at| u8::try_from(at % 16).unwrap()));
        view! { <AvatarGrid pixels=pixels press=Callback::new(|_: usize| ()) drag=Callback::new(|_: usize| ()) /> }.to_html()
    });
    assert_eq!(html.matches("class=\"avatar-pixel\"").count(), 256);
    assert!(html.contains("--avatar-side:16"));
    assert!(html.contains("16 by 16 avatar drawing canvas"));
    for coordinate in [
        "Pixel 1, 1",
        "Pixel 16, 1",
        "Pixel 1, 2",
        "Pixel 1, 16",
        "Pixel 16, 16",
    ] {
        assert!(html.contains(coordinate));
    }
    assert!(!html.contains("Pixel 17,"));
    assert!(include_str!("../../styles.css")
        .contains("grid-template-columns:repeat(var(--avatar-side), 1fr)"));
}

#[test]
fn brushes_and_fill_respect_16_by_16_edges_without_row_wrap() {
    let mut pixels = [0u8; Urma::AVATAR_PIXELS];
    apply_brush(&mut pixels, 15, 0, BrushShape::Square2, 4);
    assert_eq!(pixels.iter().filter(|&&value| value == 4).count(), 2);
    assert_eq!((pixels[15], pixels[31]), (4, 4));
    assert_eq!((pixels[16], pixels[32]), (0, 0));
    apply_brush(&mut pixels, 15, 15, BrushShape::Single, 15);
    assert_eq!(pixels[255], 15);
    let mut split = [0u8; Urma::AVATAR_PIXELS];
    for row in 0..16 {
        split[row * 16 + 1] = 1;
    }
    flood_fill(&mut split, 0, 0, 9);
    assert_eq!(split.iter().filter(|&&value| value == 9).count(), 16);
    assert_eq!((split[15], split[16]), (0, 9));
    assert!(editor_preview(&split, 48).contains("viewBox=\"0 0 16 16\""));
    split[255] = 16;
    assert!(editor_preview(&split, 48).contains("invalid palette index"));
}

#[test]
fn stored_identity_512_uses_identicon_with_notice_and_editor_blank_without_mutation() {
    let author = AuthorId::from_bytes([0x77; 32]);
    let identity = PublicIdentity {
        nickname: Some("name".into()),
        avatar: Some(vec![0x12; 512]),
    };
    let identities = BTreeMap::from([(author.to_hex(), identity.clone())]);
    let drawn = drawn(&identities, author, 64);
    assert!(drawn.contains("Unsupported stored avatar; showing identity pattern."));
    assert_eq!(
        drawn.replace(
            "<title>Unsupported stored avatar; showing identity pattern.</title>",
            ""
        ),
        render_identicon_svg(&author, 64)
    );
    let owner = Owner::new();
    owner.with(|| {
        let notice = RwSignal::new(Said::Nothing);
        let pixels = confirmed_pixels(&identity, notice);
        assert_eq!(pixels, [AVATAR_BACKGROUND; Urma::AVATAR_PIXELS]);
        assert!(notice.with_untracked(|notice| matches!(notice, Said::Something(reason) if reason.contains("stored bytes are unchanged"))));
        let current = PublicIdentity { nickname: None, avatar: Some(vec![0xef; 128]) };
        let pixels = confirmed_pixels(&current, notice);
        assert_eq!((pixels[0], pixels[1], pixels[254], pixels[255]), (14, 15, 14, 15));
        assert!(notice.with_untracked(|notice| matches!(notice, Said::Nothing)));
    });
    assert_eq!(identity.avatar, Some(vec![0x12; 512]));
    assert_eq!(identities[&author.to_hex()], identity);
}

#[test]
fn current_stored_identity_uses_the_16_pixel_renderer_without_legacy_notice() {
    let author = AuthorId::from_bytes([0x77; 32]);
    let identity = PublicIdentity {
        nickname: None,
        avatar: Some(vec![0x12; 128]),
    };
    let identities = BTreeMap::from([(author.to_hex(), identity)]);
    let svg = drawn(&identities, author, 64);
    assert!(svg.contains("viewBox=\"0 0 16 16\""));
    assert!(!svg.contains("Unsupported"));
}
