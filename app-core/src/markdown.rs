use crate::config::MAX_IMAGE_URL_BYTES;
use crate::errors::{ImageSizeError, ImageUrlError};
use pulldown_cmark::{CowStr, Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FetchedImage {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

impl FetchedImage {
    pub fn display_bytes(&self) -> Result<usize, ImageSizeError> {
        let width = usize::try_from(self.width)?;
        let height = usize::try_from(self.height)?;
        let Some(pixels) = width.checked_mul(height) else {
            return Err(ImageSizeError::Overflow);
        };
        let Some(rgba) = pixels.checked_mul(4) else {
            return Err(ImageSizeError::Overflow);
        };
        rgba.checked_add(self.data_url.len())
            .ok_or(ImageSizeError::Overflow)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Image { url: String, alt: String },
}

enum Reading {
    Plain,
    Image { url: String, alt: String },
}

struct Segments {
    done: Vec<Segment>,
    plain: String,
    reading: Reading,
}

impl Segments {
    fn open_image(&mut self, url: CowStr<'_>) {
        if !self.plain.is_empty() {
            self.done
                .push(Segment::Text(std::mem::take(&mut self.plain)));
        }
        self.reading = Reading::Image {
            url: url.into_string(),
            alt: String::new(),
        };
    }

    fn close_image(&mut self) {
        match std::mem::replace(&mut self.reading, Reading::Plain) {
            Reading::Image { url, alt } => self.done.push(Segment::Image { url, alt }),
            Reading::Plain => (),
        }
    }

    fn text(&mut self, text: &str) {
        match &mut self.reading {
            Reading::Image { alt, .. } => alt.push_str(text),
            Reading::Plain => self.plain.push_str(text),
        }
    }

    fn finished(mut self) -> Vec<Segment> {
        if !self.plain.is_empty() {
            self.done.push(Segment::Text(self.plain));
        }
        self.done
    }
}

pub fn segments(text: &str) -> Vec<Segment> {
    let mut read = Segments {
        done: Vec::new(),
        plain: String::new(),
        reading: Reading::Plain,
    };
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => read.open_image(dest_url),
            Event::End(TagEnd::Image) => read.close_image(),
            Event::Text(text) | Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                read.text(&text)
            }
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock,
            ) => read.text("\n"),
            _structure => (),
        }
    }
    read.finished()
}

pub fn image_url(value: &str) -> Result<Url, ImageUrlError> {
    if value.len() > MAX_IMAGE_URL_BYTES {
        return Err(ImageUrlError::TooLong {
            limit: MAX_IMAGE_URL_BYTES,
        });
    }
    let url = Url::parse(value)?;
    let web = matches!(url.scheme(), "http" | "https");
    if !web || !url.has_host() || !url.username().is_empty() || carries_password(&url) {
        return Err(ImageUrlError::Unsafe);
    }
    Ok(url)
}

fn carries_password(url: &Url) -> bool {
    let Some(_password) = url.password() else {
        return false;
    };
    true
}

pub fn browser_image_error(value: &str, secure_context: bool) -> String {
    match image_url(value) {
        Err(error) => {
            tracing::warn!(error = %error, "image link refused before any request");
            error.to_string()
        }
        Ok(url) if secure_context && url.scheme() == "http" => "HTTP image blocked by browser mixed-content policy. Use the desktop app for checked HTTP fetching.".into(),
        Ok(_checked) => "External image fetching is disabled in this browser: it cannot enforce destination and redirect checks. No proxy is used.".into(),
    }
}
