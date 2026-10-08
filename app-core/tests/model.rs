use wire_app_core::avatar::{
    decode_avatar_pixels, render_avatar_svg, render_identicon_svg, AvatarPalette,
};
use wire_app_core::errors::IdError;
use wire_app_core::model::{AuthorId, Txid};

#[test]
fn author_id_fingerprint_and_hex_roundtrip() {
    let author = AuthorId::from_bytes([0xab; 32]);
    assert_eq!(author.fingerprint(), "abababab");
    assert_eq!(author.to_hex().len(), 64);
    assert!(author.to_hex().starts_with("abab"));
    assert_eq!(AuthorId::from_hex(&author.to_hex()).unwrap(), author);
    assert!(matches!(AuthorId::from_hex("zz"), Err(IdError::Hex(_))));
    assert_eq!(
        AuthorId::from_hex("abab"),
        Err(IdError::Length {
            expected: 32,
            got: 2
        })
    );
}

#[test]
fn txid_display_hex_reverses_wire_order() {
    let mut raw = [0u8; 32];
    raw[0] = 0x01;
    raw[31] = 0xfe;
    let txid = Txid::from_raw_bytes(raw);
    assert!(hex::encode(txid.0).starts_with("01"));
    let display = txid.to_display_hex();
    assert!(display.starts_with("fe"));
    assert!(display.ends_with("01"));
    assert_eq!(Txid::from_display_hex(&display).unwrap(), txid);
    assert_eq!(
        txid.short_display(),
        format!("{}...{}", &display[..8], &display[58..])
    );
}

#[test]
fn avatar_palette_and_rendering() {
    assert_eq!(AvatarPalette::COLORS.len(), 16);
    assert_eq!(AvatarPalette::hex(0), "#000000");
    assert_eq!(AvatarPalette::hex(15), "#FFFFFF");
    assert_eq!(AvatarPalette::hex(0x1F), "#FFFFFF");

    let mut bytes = [0u8; 128];
    bytes[0] = 0x1F;
    let pixels = decode_avatar_pixels(&bytes);
    assert_eq!(pixels[0], 1);
    assert_eq!(pixels[1], 15);

    let svg = render_avatar_svg(&bytes, 64);
    assert!(svg.contains("<svg"));
    assert!(svg.contains(r#"viewBox="0 0 16 16""#));
    assert!(svg.contains(r#"width="64""#));

    let identicon = render_identicon_svg(&AuthorId::from_bytes([0x77; 32]), 64);
    assert!(identicon.contains("<svg"));
    assert!(identicon.contains(r#"viewBox="0 0 32 32""#));
}
