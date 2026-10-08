use crate::model::AuthorId;
pub use urma_core::avatar::{
    decode_pixels as decode_avatar_pixels, encode_pixels as encode_avatar_pixels,
};
pub use urma_core::format::Urma;

pub struct AvatarPalette;

impl AvatarPalette {
    pub const COLORS: [&'static str; 16] = Urma::AVATAR_PALETTE;

    pub fn hex(index: u8) -> &'static str {
        Self::COLORS[usize::from(index & 0x0F)]
    }
}

pub fn render_avatar_svg(bytes: &[u8; Urma::AVATAR_BYTES], size: u32) -> String {
    let pixels = decode_avatar_pixels(bytes);
    let mut rects = String::with_capacity(4096);
    for y in 0..Urma::AVATAR_SIDE {
        let mut x = 0;
        while x < Urma::AVATAR_SIDE {
            let color_index = pixels[y * Urma::AVATAR_SIDE + x];
            let color = AvatarPalette::hex(color_index);
            let mut width = 1;
            while x + width < Urma::AVATAR_SIDE
                && pixels[y * Urma::AVATAR_SIDE + x + width] == color_index
            {
                width += 1;
            }
            rects.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{width}" height="1" fill="{color}" />"#
            ));
            x += width;
        }
    }
    framed(&rects, size, Urma::AVATAR_SIDE)
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
    framed(&rects, size, 32)
}

fn framed(rects: &str, size: u32, side: usize) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {side} {side}" width="{size}" height="{size}" shape-rendering="crispEdges">{rects}</svg>"#
    )
}
