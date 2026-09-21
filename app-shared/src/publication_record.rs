use urma_core::{format::PublicRecord, topics::Topics};

/// The same record construction feeds both existing atomic publishers.
pub fn build(
    kind: &str,
    text: String,
    target: Option<String>,
    wire: String,
    hashtags: Vec<String>,
    avatar: Option<Vec<u8>>,
) -> Result<PublicRecord, String> {
    let topics = Topics { wire, hashtags };
    topics.validate().map_err(|e| e.to_string())?;
    let structured = !topics.wire.is_empty() || !topics.hashtags.is_empty();
    if kind != "post" && (target.is_some() || structured) {
        return Err("Profile and avatar records cannot have a reply target or topics".into());
    }
    let record = match kind {
        "profile" if avatar.is_none() => PublicRecord::Profile(text),
        "avatar" if text.is_empty() => PublicRecord::Avatar(Box::new(
            avatar
                .ok_or("Missing avatar pixels")?
                .try_into()
                .map_err(|_| "An avatar must contain exactly 512 pixel bytes")?,
        )),
        "post" if avatar.is_none() => {
            if text.trim().is_empty() {
                return Err("Write a post or reply first".into());
            }
            match (target, structured) {
                (Some(target), true) => PublicRecord::WireReply {
                    target: target.parse().map_err(|_| "Invalid reply target")?,
                    topics,
                    text,
                },
                (None, true) => PublicRecord::WirePost { topics, text },
                (Some(target), false) => PublicRecord::Reply {
                    target: target.parse().map_err(|_| "Invalid reply target")?,
                    text,
                },
                (None, false) => PublicRecord::Post(text),
            }
        }
        _ => return Err("Unsupported publication fields".into()),
    };
    record.encode().map_err(|e| e.to_string())?;
    Ok(record)
}

/// Only decoded, verified signed bytes supply approval content.
pub struct Review {
    pub kind: String,
    pub text: String,
    pub target: Option<String>,
    pub wire: String,
    pub hashtags: Vec<String>,
    pub avatar: Option<Vec<u8>>,
}

pub fn review(bytes: &[u8]) -> Result<Review, String> {
    let mut result = Review {
        kind: "post".into(),
        text: String::new(),
        target: None,
        wire: String::new(),
        hashtags: vec![],
        avatar: None,
    };
    match PublicRecord::decode(bytes).map_err(|e| e.to_string())? {
        PublicRecord::Post(text) => result.text = text,
        PublicRecord::Reply { text, target } => {
            result.text = text;
            result.target = Some(target.to_string());
        }
        PublicRecord::WirePost { text, topics } => {
            result.text = text;
            result.wire = topics.wire;
            result.hashtags = topics.hashtags;
        }
        PublicRecord::WireReply {
            text,
            topics,
            target,
        } => {
            result.text = text;
            result.wire = topics.wire;
            result.hashtags = topics.hashtags;
            result.target = Some(target.to_string());
        }
        PublicRecord::Profile(text) => {
            result.kind = "profile".into();
            result.text = text;
        }
        PublicRecord::Avatar(pixels) => {
            result.kind = "avatar".into();
            result.avatar = Some(pixels.to_vec());
        }
    }
    Ok(result)
}
