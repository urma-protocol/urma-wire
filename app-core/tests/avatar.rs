use wire_app_core::avatar::{
    decode_avatar_pixels, encode_avatar_pixels, render_avatar_svg, render_identicon_svg,
    AvatarPalette, Urma,
};
use wire_app_core::model::AuthorId;

#[test]
fn canonical_dimensions_and_every_ega_colour_are_exact() {
    assert_eq!(
        (Urma::AVATAR_SIDE, Urma::AVATAR_PIXELS, Urma::AVATAR_BYTES),
        (16, 256, 128)
    );
    let expected = [
        "#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA",
        "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF",
    ];
    assert_eq!(AvatarPalette::COLORS, expected);
    for (index, colour) in expected.iter().enumerate() {
        assert_eq!(AvatarPalette::hex(u8::try_from(index).unwrap()), *colour);
    }
}

#[test]
fn all_palette_indices_roundtrip_in_row_major_high_nibble_order() {
    let pixels = std::array::from_fn(|index| u8::try_from(index % 16).unwrap());
    let packed = encode_avatar_pixels(&pixels).unwrap();
    for row in 0..16 {
        assert_eq!(
            &packed[row * 8..row * 8 + 8],
            &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
        );
    }
    assert_eq!(decode_avatar_pixels(&packed), pixels);
    let svg = render_avatar_svg(&packed, 64);
    assert!(svg.contains(r#"viewBox="0 0 16 16""#));
    assert_eq!(svg.matches("<rect ").count(), 256);
    for colour in AvatarPalette::COLORS {
        assert_eq!(svg.matches(&format!("fill=\"{colour}\"")).count(), 16);
    }
}

#[test]
fn first_and_last_rows_and_nibbles_have_the_expected_svg_positions() {
    let mut bytes = [0u8; 128];
    bytes[0] = 0x12;
    bytes[7] = 0x34;
    bytes[8] = 0x56;
    bytes[119] = 0x78;
    bytes[120] = 0x9a;
    bytes[127] = 0xef;
    let pixels = decode_avatar_pixels(&bytes);
    let svg = render_avatar_svg(&bytes, 41);
    for (index, colour) in [
        (0, 1),
        (1, 2),
        (14, 3),
        (15, 4),
        (16, 5),
        (17, 6),
        (238, 7),
        (239, 8),
        (240, 9),
        (241, 10),
        (254, 14),
        (255, 15),
    ] {
        assert_eq!(pixels[index], colour);
        assert!(svg.contains(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill=\"{}\" />",
            index % 16,
            index / 16,
            AvatarPalette::hex(colour),
        )));
    }
    assert_eq!(encode_avatar_pixels(&pixels).unwrap(), bytes);
}

#[test]
fn runs_stop_at_row_boundaries_and_identicon_geometry_is_unchanged() {
    let avatar = render_avatar_svg(&[0xff; 128], 64);
    assert_eq!(avatar.matches("<rect ").count(), 16);
    assert!(avatar.contains(r##"x="0" y="15" width="16" height="1" fill="#FFFFFF""##));
    let identicon = render_identicon_svg(&AuthorId::from_bytes([0x77; 32]), 64);
    assert!(identicon.contains(r#"viewBox="0 0 32 32""#));
    assert!(identicon.contains(r##"width="32" height="32" fill="#AAAAAA" rx="4""##));
    assert!(identicon.contains(r#"x="28" y="28" width="4" height="4""#));
}

#[test]
fn arbitrary_packed_bytes_are_pixels_even_with_image_magic() {
    for magic in [
        b"\x89PNG\r\n\x1a\n".as_slice(),
        b"GIF89a",
        b"\xff\xd8\xff",
        b"RIFF",
        b"<svg onload=",
    ] {
        let mut bytes = [0u8; 128];
        bytes[..magic.len()].copy_from_slice(magic);
        let pixels = decode_avatar_pixels(&bytes);
        assert_eq!(pixels[0], magic[0] >> 4);
        assert_eq!(pixels[1], magic[0] & 15);
        assert_eq!(encode_avatar_pixels(&pixels).unwrap(), bytes);
        let svg = render_avatar_svg(&bytes, 16);
        assert!(svg.contains(r#"viewBox="0 0 16 16""#));
        assert!(!svg.contains("onload"));
        assert!(!svg.contains("<image"));
    }
}

#[test]
fn checked_encoder_rejects_invalid_indices_and_legacy_size_is_not_renderable() {
    for value in [16, 31, 255] {
        let mut pixels = [0u8; 256];
        pixels[255] = value;
        assert!(encode_avatar_pixels(&pixels).is_err());
    }
    for length in [0, 127, 129, 136, 512] {
        assert!(<&[u8; Urma::AVATAR_BYTES]>::try_from(vec![0; length].as_slice()).is_err());
    }
}
