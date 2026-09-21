use crate::model::AuthorId;

/// Reference 16-color RGB palette defined in WIRE.md.
pub struct AvatarPalette;

impl AvatarPalette {
    pub const COLORS: [&'static str; 16] = [
        "#000000", // 0
        "#0000AA", // 1
        "#00AA00", // 2
        "#00AAAA", // 3
        "#AA0000", // 4
        "#AA00AA", // 5
        "#AA5500", // 6
        "#AAAAAA", // 7
        "#555555", // 8
        "#5555FF", // 9
        "#55FF55", // A
        "#55FFFF", // B
        "#FF5555", // C
        "#FF55FF", // D
        "#FFFF55", // E
        "#FFFFFF", // F
    ];

    pub fn hex(index: u8) -> &'static str {
        Self::COLORS[(index & 0x0F) as usize]
    }
}

/// Decodes 512 raw bytes (32x32 pixels, 4 bpp row-major) into 1024 palette indices.
/// High nibble is first pixel, low nibble is second pixel.
pub fn decode_avatar_pixels(bytes: &[u8; 512]) -> [u8; 1024] {
    let mut pixels = [0u8; 1024];
    for (i, &byte) in bytes.iter().enumerate() {
        pixels[i * 2] = (byte >> 4) & 0x0F;
        pixels[i * 2 + 1] = byte & 0x0F;
    }
    pixels
}

/// Encodes 1024 palette indices (0..=15) into 512 raw bytes.
pub fn encode_avatar_pixels(pixels: &[u8; 1024]) -> [u8; 512] {
    let mut bytes = [0u8; 512];
    for (i, chunk) in pixels.as_chunks::<2>().0.iter().enumerate() {
        bytes[i] = ((chunk[0] & 0x0F) << 4) | (chunk[1] & 0x0F);
    }
    bytes
}

/// Renders a 32x32 pixel avatar as a clean, responsive SVG.
/// Uses run-length optimization along each row to minimize SVG node count.
pub fn render_avatar_svg(bytes: &[u8; 512], size: u32) -> String {
    let pixels = decode_avatar_pixels(bytes);
    let mut rects = String::with_capacity(4096);

    for y in 0..32 {
        let mut x = 0;
        while x < 32 {
            let color_idx = pixels[y * 32 + x];
            let color = AvatarPalette::hex(color_idx);
            let mut width = 1;
            while x + width < 32 && pixels[y * 32 + x + width] == color_idx {
                width += 1;
            }
            rects.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{width}" height="1" fill="{color}" />"#
            ));
            x += width;
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="{size}" height="{size}" shape-rendering="crispEdges">{rects}</svg>"#
    )
}

/// Renders a deterministic fallback identicon SVG for an author without an on-chain avatar.
/// Generated from the author's public key using a vertically symmetric 8x8 pattern
/// rendered inside a 32x32 frame with the URMA Wire palette.
pub fn render_identicon_svg(author: &AuthorId, size: u32) -> String {
    let bytes = author.as_bytes();
    let bg_color = AvatarPalette::hex(bytes[0] % 16);
    let fg_color = AvatarPalette::hex((bytes[1] % 15) + 1);
    let accent_color = AvatarPalette::hex((bytes[2] % 15) + 1);

    let mut rects =
        format!(r#"<rect x="0" y="0" width="32" height="32" fill="{bg_color}" rx="4" />"#);

    // 8x8 grid scaled by 4x4 pixels each, horizontally symmetric (cols 0..4 mirrored to 4..8)
    for y in 0..8 {
        for x in 0..4 {
            let byte_idx = (y * 4 + x) % 32;
            let bit = (bytes[byte_idx] >> (x % 8)) & 1;
            if bit == 1 {
                let color = if (bytes[byte_idx] & 2) != 0 {
                    accent_color
                } else {
                    fg_color
                };
                let px1 = x * 4;
                let px2 = (7 - x) * 4;
                let py = y * 4;
                rects.push_str(&format!(
                    r#"<rect x="{px1}" y="{py}" width="4" height="4" fill="{color}" />"#
                ));
                if px1 != px2 {
                    rects.push_str(&format!(
                        r#"<rect x="{px2}" y="{py}" width="4" height="4" fill="{color}" />"#
                    ));
                }
            }
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="{size}" height="{size}" shape-rendering="crispEdges">{rects}</svg>"#
    )
}
