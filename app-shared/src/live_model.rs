use crate::model::Post;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Publication {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub avatar: Option<Vec<u8>>,
    pub id: String,
    pub txid: String,
    pub text: String,
    #[serde(default)]
    pub wire: String,
    #[serde(default)]
    pub hashtags: Vec<String>,
    pub target: Option<String>,
    pub fee: u64,
    pub selected_funding: u64,
    pub returned: u64,
    pub approved: bool,
    pub confirmed: usize,
    pub mempool: usize,
    pub message: String,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feed {
    #[serde(default)]
    pub identities: std::collections::BTreeMap<String, crate::public_identity::PublicIdentity>,
    pub posts: Vec<Post>,
    pub start: u64,
    pub tip: u64,
    pub complete_to_tip: bool,
}

pub fn matches_topic(post: &Post, label: &str) -> bool {
    if let Some(wire) = label.strip_prefix('/') {
        !wire.is_empty() && post.wire == wire
    } else if let Some(tag) = label.strip_prefix('#') {
        post.hashtags.iter().any(|value| value == tag)
    } else {
        false
    }
}

pub fn matches_topic_filters(post: &Post, wire: &str, hashtag: &str) -> bool {
    (wire.is_empty() || post.wire == wire)
        && (hashtag.is_empty() || post.hashtags.iter().any(|tag| tag == hashtag))
}
