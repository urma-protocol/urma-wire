use crate::config::{
    IMAGE_CONNECT_TIMEOUT_SECS, IMAGE_TIMEOUT_SECS, MAX_IMAGE_ALLOC_BYTES, MAX_IMAGE_DIMENSION,
    MAX_IMAGE_DOWNLOAD_BYTES, MAX_IMAGE_PIXELS, MAX_IMAGE_RENDER_BYTES, SUPPORTED_IMAGE_FORMATS,
};
use crate::errors::MediaError;
use base64::Engine;
use image::{ImageFormat, ImageReader};
use std::io::{Cursor, Read};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use url::Url;
use wire_app_core::markdown::{image_url, FetchedImage};

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _last] = ip.octets();
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

fn decode_image(mime: &str, bytes: &[u8]) -> Result<FetchedImage, MediaError> {
    if u64::try_from(bytes.len())? > MAX_IMAGE_DOWNLOAD_BYTES {
        return Err(MediaError::TooLarge);
    }
    let Some(declared) = mime.split(';').next() else {
        return Err(MediaError::NotImage);
    };
    if !declared.trim().to_ascii_lowercase().starts_with("image/") {
        return Err(MediaError::NotImage);
    }
    let format = image::guess_format(bytes).map_err(MediaError::Signature)?;
    if !SUPPORTED_IMAGE_FORMATS.contains(&format) {
        return Err(MediaError::Format);
    }
    let (width, height) = bounded_reader(bytes, format)
        .into_dimensions()
        .map_err(MediaError::Dimensions)?;
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
    {
        return Err(MediaError::Bounds);
    }
    let decoded = bounded_reader(bytes, format)
        .decode()
        .map_err(MediaError::Decoding)?;
    let mut inert = Cursor::new(Vec::new());
    decoded
        .to_rgba8()
        .write_to(&mut inert, ImageFormat::Png)
        .map_err(MediaError::Rendering)?;
    if inert.get_ref().len() > MAX_IMAGE_RENDER_BYTES {
        return Err(MediaError::Rendered);
    }
    Ok(FetchedImage {
        width,
        height,
        data_url: format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(inert.into_inner())
        ),
    })
}

fn bounded_reader(bytes: &[u8], format: ImageFormat) -> ImageReader<Cursor<&[u8]>> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC_BYTES);
    reader.limits(limits);
    reader
}

pub fn fetch(value: &str) -> Result<FetchedImage, MediaError> {
    fetch_with(value, resolve, pinned_request)
}

fn resolve(url: &Url) -> Result<Vec<SocketAddr>, MediaError> {
    let Some(host) = url.host_str() else {
        return Err(MediaError::NoHost);
    };
    let Some(port) = url.port_or_known_default() else {
        return Err(MediaError::NoPort);
    };
    match (host.trim_matches(['[', ']']), port).to_socket_addrs() {
        Ok(addresses) => Ok(addresses.collect()),
        Err(error) => Err(MediaError::Resolution(error)),
    }
}

enum Header {
    Present(String),
    Absent,
}

struct Response {
    status: u16,
    location: Header,
    mime: Header,
    length: Option<u64>,
    body: Box<dyn Read>,
}

fn header(response: &reqwest::blocking::Response, name: reqwest::header::HeaderName) -> Header {
    let Some(value) = response.headers().get(&name) else {
        return Header::Absent;
    };
    match value.to_str() {
        Ok(text) => Header::Present(text.to_owned()),
        Err(error) => {
            tracing::warn!(error = %error, header = %name, "image header is not visible ASCII");
            Header::Absent
        }
    }
}

fn pinned_request(url: &Url, addresses: &[SocketAddr]) -> Result<Response, MediaError> {
    let Some(host) = url.host_str() else {
        return Err(MediaError::NoHost);
    };
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(IMAGE_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(IMAGE_CONNECT_TIMEOUT_SECS))
        .resolve_to_addrs(host.trim_matches(['[', ']']), addresses)
        .build()
        .map_err(MediaError::Transport)?;
    let response = client
        .get(url.clone())
        .header("Accept", "image/*")
        .header("Cache-Control", "no-cache")
        .send()
        .map_err(MediaError::Request)?;
    Ok(Response {
        status: response.status().as_u16(),
        location: header(&response, reqwest::header::LOCATION),
        mime: header(&response, reqwest::header::CONTENT_TYPE),
        length: response.content_length(),
        body: Box::new(response),
    })
}

fn fetch_with(
    value: &str,
    mut resolve: impl FnMut(&Url) -> Result<Vec<SocketAddr>, MediaError>,
    mut request: impl FnMut(&Url, &[SocketAddr]) -> Result<Response, MediaError>,
) -> Result<FetchedImage, MediaError> {
    let mut url = image_url(value)?;
    for redirects in 0..=1 {
        let addresses = resolve(&url)?;
        if addresses.is_empty()
            || addresses
                .iter()
                .any(|address| !public_address(address.ip()))
        {
            return Err(MediaError::NotPublic);
        }
        let response = request(&url, &addresses)?;
        if !(300..400).contains(&response.status) {
            return downloaded(response);
        }
        if redirects == 1 {
            return Err(MediaError::Redirects);
        }
        let Header::Present(location) = response.location else {
            return Err(MediaError::Redirect);
        };
        let target = url.join(&location).map_err(MediaError::RedirectTarget)?;
        url = image_url(target.as_str())?;
    }
    Err(MediaError::Redirects)
}

fn downloaded(response: Response) -> Result<FetchedImage, MediaError> {
    if !(200..300).contains(&response.status) {
        return Err(MediaError::Status);
    }
    for length in response.length.iter() {
        if *length > MAX_IMAGE_DOWNLOAD_BYTES {
            return Err(MediaError::TooLarge);
        }
    }
    let Header::Present(mime) = response.mime else {
        return Err(MediaError::NoMime);
    };
    let mut bytes = Vec::new();
    response
        .body
        .take(MAX_IMAGE_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(MediaError::Download)?;
    decode_image(&mime, &bytes)
}
