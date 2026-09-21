use pulldown_cmark::{Event, Parser, Tag, TagEnd};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FetchedImage {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

impl FetchedImage {
    pub fn display_bytes(&self) -> usize {
        self.data_url.len().saturating_add(
            (self.width as usize)
                .saturating_mul(self.height as usize)
                .saturating_mul(4),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Image { url: String, alt: String },
}

/// Small inert Markdown subset: text, code, paragraphs and external image references.
/// HTML is displayed as text. No parser output is passed to innerHTML.
pub fn segments(text: &str) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut plain = String::new();
    let mut image: Option<(String, String)> = None;
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                if !plain.is_empty() {
                    result.push(Segment::Text(std::mem::take(&mut plain)));
                }
                image = Some((dest_url.into_string(), String::new()));
            }
            Event::End(TagEnd::Image) => {
                if let Some((url, alt)) = image.take() {
                    result.push(Segment::Image { url, alt });
                }
            }
            Event::Text(text) | Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                if let Some((_, alt)) = &mut image {
                    alt.push_str(&text);
                } else {
                    plain.push_str(&text);
                }
            }
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock,
            ) => {
                if let Some((_, alt)) = &mut image {
                    alt.push('\n');
                } else {
                    plain.push('\n');
                }
            }
            _ => (),
        }
    }
    if !plain.is_empty() {
        result.push(Segment::Text(plain));
    }
    result
}

pub fn image_url(value: &str) -> Result<url::Url, String> {
    if value.len() > 4096 {
        return Err("Image URL exceeds 4096 bytes".into());
    }
    let url = url::Url::parse(value).map_err(|_| "Invalid image URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("External images require HTTP or HTTPS without credentials".into());
    }
    Ok(url)
}

pub fn browser_image_error(value: &str, secure_context: bool) -> String {
    match image_url(value) {
        Err(error) => error,
        Ok(url) if secure_context && url.scheme() == "http" => "HTTP image blocked by browser mixed-content policy. Use the desktop app for checked HTTP fetching.".into(),
        Ok(_) => "External image fetching is disabled in this browser: it cannot enforce destination and redirect checks. No proxy is used.".into(),
    }
}
