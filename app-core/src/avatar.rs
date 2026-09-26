use crate::model::AuthorId;

pub struct AvatarPalette;

impl AvatarPalette {
    pub const COLORS: [&'static str; 16] = [
        "#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA",
        "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF",
    ];

    pub fn hex(index: u8) -> &'static str {
        Self::COLORS[usize::from(index & 0x0F)]
    }
}

pub fn decode_avatar_pixels(bytes: &[u8; 512]) -> [u8; 1024] {
    let mut pixels = [0u8; 1024];
    for (at, &byte) in bytes.iter().enumerate() {
        pixels[at * 2] = (byte >> 4) & 0x0F;
        pixels[at * 2 + 1] = byte & 0x0F;
    }
    pixels
}

pub fn encode_avatar_pixels(pixels: &[u8; 1024]) -> [u8; 512] {
    let mut bytes = [0u8; 512];
    for (at, pair) in pixels.as_chunks::<2>().0.iter().enumerate() {
        bytes[at] = ((pair[0] & 0x0F) << 4) | (pair[1] & 0x0F);
    }
    bytes
}

pub fn render_avatar_svg(bytes: &[u8; 512], size: u32) -> String {
    let pixels = decode_avatar_pixels(bytes);
    let mut rects = String::with_capacity(4096);
    for y in 0..32 {
        let mut x = 0;
        while x < 32 {
            let color_index = pixels[y * 32 + x];
            let color = AvatarPalette::hex(color_index);
            let mut width = 1;
            while x + width < 32 && pixels[y * 32 + x + width] == color_index {
                width += 1;
            }
            rects.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{width}" height="1" fill="{color}" />"#
            ));
            x += width;
        }
    }
    framed(&rects, size)
}

pub fn render_identicon_svg(author: &AuthorId, size: u32) -> String {
    let bytes = author.as_bytes();
    let background = AvatarPalette::hex(bytes[0] % 16);
    let foreground = AvatarPalette::hex((bytes[1] % 15) + 1);
    let accent = AvatarPalette::hex((bytes[2] % 15) + 1);
    let mut rects =
        format!(r#"<rect x="0" y="0" width="32" height="32" fill="{background}" rx="4" />"#);
    for y in 0..8 {
        for x in 0..4 {
            let at = (y * 4 + x) % 32;
            if (bytes[at] >> (x % 8)) & 1 != 1 {
                continue;
            }
            let color = match bytes[at] & 2 {
                0 => foreground,
                _set => accent,
            };
            let left = x * 4;
            let right = (7 - x) * 4;
            let top = y * 4;
            rects.push_str(&format!(
                r#"<rect x="{left}" y="{top}" width="4" height="4" fill="{color}" />"#
            ));
            if left != right {
                rects.push_str(&format!(
                    r#"<rect x="{right}" y="{top}" width="4" height="4" fill="{color}" />"#
                ));
            }
        }
    }
    framed(&rects, size)
}

fn framed(rects: &str, size: u32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="{size}" height="{size}" shape-rendering="crispEdges">{rects}</svg>"#
    )
}
