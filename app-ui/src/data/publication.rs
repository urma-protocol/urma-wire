use crate::data::engine::ask;
use crate::data::failure::Failure;
use crate::limits::MAX_POST_BYTES;
use crate::limits::MAX_REPLY_BYTES;
use serde_json::json;
use wire_app_shared::live_model::Publication;
use wire_app_shared::Chain;
use wire_app_shared::Txid;

pub enum Pending {
    Awaiting(Publication),
    Settled,
}

#[derive(Clone, Copy)]
pub enum Addressed {
    Thread(Txid),
    Standalone,
}

impl Addressed {
    fn target_hex(&self) -> Option<String> {
        match self {
            Addressed::Thread(txid) => Some(txid.to_display_hex()),
            Addressed::Standalone => None,
        }
    }

    pub fn max_bytes(&self) -> usize {
        match self {
            Addressed::Thread(_) => MAX_REPLY_BYTES,
            Addressed::Standalone => MAX_POST_BYTES,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Addressed::Thread(_) => "Reply text",
            Addressed::Standalone => "Post text",
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            Addressed::Thread(_) => "Write a reply…",
            Addressed::Standalone => "What's on your mind?",
        }
    }
}

pub struct Draft {
    pub text: String,
    pub wire: String,
    pub hashtags: Vec<String>,
    pub addressed: Addressed,
}

impl Draft {
    pub fn spoken(text: String, wire: &str, hashtags: &str, addressed: Addressed) -> Self {
        let mut hashtags: Vec<String> = hashtags
            .split_whitespace()
            .map(|tag| tag.trim_start_matches('#').to_ascii_lowercase())
            .collect();
        hashtags.sort();
        hashtags.dedup();
        Self {
            text,
            wire: wire.trim().trim_start_matches('/').to_ascii_lowercase(),
            hashtags,
            addressed,
        }
    }
}

pub enum Profile {
    Nickname(String),
    Avatar(Vec<u8>),
}

impl Profile {
    fn kind(&self) -> &'static str {
        match self {
            Profile::Nickname(_) => "profile",
            Profile::Avatar(_) => "avatar",
        }
    }

    fn text(&self) -> String {
        match self {
            Profile::Nickname(nickname) => nickname.clone(),
            Profile::Avatar(_) => String::new(),
        }
    }

    fn pixels(&self) -> Option<Vec<u8>> {
        match self {
            Profile::Nickname(_) => None,
            Profile::Avatar(pixels) => Some(pixels.clone()),
        }
    }
}

pub async fn quote(chain: Chain, draft: Draft) -> Result<Publication, Failure> {
    ask(
        "quote_publication",
        &json!({
            "chain": chain.code(),
            "text": draft.text,
            "wire": draft.wire,
            "hashtags": draft.hashtags,
            "target": draft.addressed.target_hex(),
        }),
    )
    .await
}

pub async fn quote_profile(chain: Chain, profile: Profile) -> Result<Publication, Failure> {
    ask(
        "quote_publication",
        &json!({
            "chain": chain.code(),
            "kind": profile.kind(),
            "text": profile.text(),
            "avatar": profile.pixels(),
            "target": null,
            "wire": "",
            "hashtags": [],
        }),
    )
    .await
}

pub async fn pending() -> Result<Pending, Failure> {
    let found: Option<Publication> = ask("publication_status", &json!({})).await?;
    let Some(publication) = found else {
        return Ok(Pending::Settled);
    };
    Ok(Pending::Awaiting(publication))
}

pub async fn advance(id: &str, approve: bool) -> Result<Publication, Failure> {
    ask(
        "advance_publication",
        &json!({ "id": id, "approve": approve }),
    )
    .await
}
