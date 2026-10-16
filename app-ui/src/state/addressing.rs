use wire_app_core::model::{Post, Txid};
use wire_app_core::publication::Addressed;

#[derive(Clone)]
pub enum Speaking {
    InReplyTo(Post),
    Freely,
}

impl Speaking {
    pub fn addressed(&self) -> Addressed {
        match self {
            Speaking::InReplyTo(post) => Addressed::Thread(post.txid),
            Speaking::Freely => Addressed::Standalone,
        }
    }

    pub fn draft_key(&self) -> String {
        match self {
            Speaking::InReplyTo(post) => post.txid.to_display_hex(),
            Speaking::Freely => String::new(),
        }
    }

    pub fn replies_to(&self, txid: Txid) -> bool {
        match self {
            Speaking::InReplyTo(post) => post.txid == txid,
            Speaking::Freely => false,
        }
    }

    pub fn claims_focus(&self) -> bool {
        match self {
            Speaking::InReplyTo(_) => true,
            Speaking::Freely => false,
        }
    }

    pub fn publish_label(&self) -> &'static str {
        match self {
            Speaking::InReplyTo(_) => "Reply",
            Speaking::Freely => "Publish",
        }
    }

    pub fn text_label(&self) -> &'static str {
        match self {
            Speaking::InReplyTo(_) => "Reply text",
            Speaking::Freely => "Post text",
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            Speaking::InReplyTo(_) => "Write a reply…",
            Speaking::Freely => "What's on your mind?",
        }
    }
}
