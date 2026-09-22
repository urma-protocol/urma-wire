// Checked native transport. No disk, HTTP or application media cache is retained.
use crate::markdown::FetchedImage;
use base64::Engine;
use image::{ImageFormat, ImageReader};
use std::{
    io::{Cursor, Read},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    time::Duration,
};

/// Raster formats with a decoder compiled in. Everything decodes to inert RGBA
/// and is re-encoded as PNG, so animation and source metadata never survive.
const SUPPORTED_FORMATS: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::WebP,
    ImageFormat::Gif,
    ImageFormat::Bmp,
    ImageFormat::Ico,
    ImageFormat::Tiff,
    ImageFormat::Avif,
];

const MAX_DOWNLOAD_BYTES: usize = 4 * 1024 * 1024;
const MAX_PIXELS: u64 = 4 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;
const MAX_RENDER_BYTES: usize = 20 * 1024 * 1024;

/// Conservative public-unicast allowlist, rejecting reserved and transition ranges.
fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 0 || b == 168 || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
                && s[0] != 0x3fff
        }
    }
}

fn decode_image(mime: &str, bytes: &[u8]) -> Result<FetchedImage, String> {
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err("Image download exceeds 4 MiB".into());
    }
    // The declared type only has to claim an image; the actual decoder comes from
    // the signature, so a mislabelled or transcoding host cannot pick the codec.
    let declared = mime.split(';').next().unwrap_or("").trim();
    if !declared.to_ascii_lowercase().starts_with("image/") {
        return Err("Image response is not an image type".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "Unrecognized image signature")?;
    if !SUPPORTED_FORMATS.contains(&format) {
        return Err("Unsupported image format".into());
    }
    let (width, height) = bounded_reader(bytes, format)
        .into_dimensions()
        .map_err(|_| "Invalid image dimensions")?;
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err("Image exceeds 4096-pixel dimension or 4-megapixel bounds".into());
    }
    let decoded = bounded_reader(bytes, format)
        .decode()
        .map_err(|_| "Image failed bounded decoding")?;
    // Re-encode inert pixels, dropping source metadata and animation.
    let mut output = Cursor::new(Vec::new());
    decoded
        .to_rgba8()
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| "Image rendering failed")?;
    if output.get_ref().len() > MAX_RENDER_BYTES {
        return Err("Rendered image exceeds bounds".into());
    }
    Ok(FetchedImage {
        width,
        height,
        data_url: format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(output.into_inner())
        ),
    })
}

fn bounded_reader(bytes: &[u8], format: ImageFormat) -> ImageReader<Cursor<&[u8]>> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    reader
}

pub fn fetch(value: &str) -> Result<FetchedImage, String> {
    fetch_with(
        value,
        |url| {
            let host = url
                .host_str()
                .ok_or("Missing image host")?
                .trim_matches(['[', ']']);
            let port = url.port_or_known_default().ok_or("Missing image port")?;
            (host, port)
                .to_socket_addrs()
                .map(|addresses| addresses.collect())
                .map_err(|_| "Image host resolution failed".into())
        },
        checked_request,
    )
}

struct Response {
    status: u16,
    location: Option<String>,
    mime: Option<String>,
    length: Option<u64>,
    body: Box<dyn Read>,
}

fn checked_request(url: &url::Url, addresses: &[SocketAddr]) -> Result<Response, String> {
    let host = url
        .host_str()
        .ok_or("Missing image host")?
        .trim_matches(['[', ']']);
    // Pin the checked addresses. Disable environment proxies and redirects so
    // no connection can resolve or forward to an unchecked destination.
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(|_| "Cannot initialize checked image transport")?;
    let response = client
        .get(url.clone())
        .header("Accept", "image/*")
        .header("Cache-Control", "no-cache")
        .send()
        .map_err(|_| "Image request failed")?;
    Ok(Response {
        status: response.status().as_u16(),
        location: response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        mime: response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        length: response.content_length(),
        body: Box::new(response),
    })
}

fn fetch_with(
    value: &str,
    mut resolve: impl FnMut(&url::Url) -> Result<Vec<SocketAddr>, String>,
    mut request: impl FnMut(&url::Url, &[SocketAddr]) -> Result<Response, String>,
) -> Result<FetchedImage, String> {
    let mut url = crate::markdown::image_url(value)?;
    for redirects in 0..=1 {
        let addresses = resolve(&url)?;
        if addresses.is_empty()
            || addresses
                .iter()
                .any(|address| !public_address(address.ip()))
        {
            return Err("Image destination is not public unicast".into());
        }
        let response = request(&url, &addresses)?;
        if (300..400).contains(&response.status) {
            if redirects == 1 {
                return Err("Image exceeds one redirect".into());
            }
            let location = response
                .location
                .as_deref()
                .ok_or("Invalid image redirect")?;
            url = crate::markdown::image_url(
                url.join(location)
                    .map_err(|_| "Invalid image redirect")?
                    .as_str(),
            )?;
            continue;
        }
        if !(200..300).contains(&response.status) {
            return Err("Image host returned an unsuccessful response".into());
        }
        if response
            .length
            .is_some_and(|length| length > MAX_DOWNLOAD_BYTES as u64)
        {
            return Err("Image download exceeds 4 MiB".into());
        }
        let mime = response.mime.ok_or("Image response has no MIME type")?;
        let mut bytes = Vec::new();
        response
            .body
            .take(MAX_DOWNLOAD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Image download failed")?;
        return decode_image(&mime, &bytes);
    }
    Err("Image exceeds one redirect".into())
}
