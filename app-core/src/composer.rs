use crate::config::MAX_IMAGE_URL_BYTES;
use crate::markdown::{image_url, segments, Segment};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Photo {
    pub url: String,
    pub alt: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PhotoError {
    #[error("Enter an HTTPS image URL.")]
    Https,
    #[error("Enter alt text describing the photo.")]
    Alt,
    #[error("Alt text cannot contain line breaks or control characters.")]
    AltControl,
    #[error("Image URL exceeds {MAX_IMAGE_URL_BYTES} bytes.")]
    UrlLength,
    #[error(
        "Image URL or alt text cannot be represented safely. Try a simpler URL or description."
    )]
    Syntax,
    #[error("This post can display at most {limit} photos. Remove one before adding another.")]
    Count { limit: usize },
    #[error("This post or reply exceeds its {limit}-byte text limit.")]
    Bytes { limit: usize },
}

impl Photo {
    pub fn new(url: &str, alt: &str) -> Result<Self, PhotoError> {
        let entered = url.trim();
        let parsed = image_url(entered).map_err(|error| match error {
            crate::errors::ImageUrlError::TooLong { .. } => PhotoError::UrlLength,
            _ => PhotoError::Https,
        })?;
        if parsed.scheme() != "https" {
            return Err(PhotoError::Https);
        }
        let url = parsed.to_string();
        if url.len() > MAX_IMAGE_URL_BYTES {
            return Err(PhotoError::UrlLength);
        }
        if url.contains(['<', '>']) {
            return Err(PhotoError::Syntax);
        }
        if alt.trim().is_empty() {
            return Err(PhotoError::Alt);
        }
        if alt.chars().any(char::is_control) {
            return Err(PhotoError::AltControl);
        }
        let photo = Self {
            url,
            alt: alt.to_owned(),
        };
        let encoded = photo.markdown();
        let images: Vec<_> = segments(&encoded)
            .into_iter()
            .filter_map(|piece| match piece {
                Segment::Image { url, alt } => Some((url, alt)),
                Segment::Text(_) => None,
            })
            .collect();
        if images != vec![(photo.url.clone(), photo.alt.clone())] {
            return Err(PhotoError::Syntax);
        }
        Ok(photo)
    }

    pub fn markdown(&self) -> String {
        let mut alt = String::new();
        for ch in self.alt.chars() {
            match ch {
                '&' => alt.push_str("&amp;"),
                '<' => alt.push_str("&lt;"),
                '>' => alt.push_str("&gt;"),
                '\\' | '[' | ']' | '*' | '_' | '`' | '!' => {
                    alt.push('\\');
                    alt.push(ch);
                }
                other => alt.push(other),
            }
        }
        format!("![{alt}](<{}>)", self.url)
    }
}

pub fn compose(
    text: &str,
    photos: &[Photo],
    max_bytes: usize,
    max_photos: usize,
) -> Result<String, PhotoError> {
    if !photos.is_empty() {
        let existing = segments(text)
            .into_iter()
            .filter(|part| matches!(part, Segment::Image { .. }))
            .count();
        if existing.saturating_add(photos.len()) > max_photos {
            return Err(PhotoError::Count { limit: max_photos });
        }
    }
    let mut result = text.to_owned();
    for photo in photos {
        if !result.is_empty() {
            result.push_str(if result.ends_with('\n') { "\n" } else { "\n\n" });
        }
        result.push_str(&photo.markdown());
    }
    if result.len() > max_bytes {
        return Err(PhotoError::Bytes { limit: max_bytes });
    }
    Ok(result)
}
