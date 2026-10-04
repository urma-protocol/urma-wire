use crate::config::{MAX_POST_BYTES, MAX_REPLY_BYTES};
use crate::model::Txid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Addressed {
    Thread(Txid),
    Standalone,
}

impl Addressed {
    pub fn max_bytes(&self) -> usize {
        match self {
            Addressed::Thread(_target) => MAX_REPLY_BYTES,
            Addressed::Standalone => MAX_POST_BYTES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    Post {
        text: String,
        wire: String,
        hashtags: Vec<String>,
        addressed: Addressed,
    },
    Nickname(String),
    Avatar(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publication {
    pub content: Content,
    pub id: String,
    pub txid: String,
    pub fee: u64,
    pub selected_funding: u64,
    pub returned: u64,
    pub approved: bool,
    pub confirmed: usize,
    pub mempool: usize,
    pub message: String,
    pub complete: bool,
    pub abandonable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Pending {
    Awaiting(Publication),
    Settled,
}
