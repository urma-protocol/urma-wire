use crate::data::publication::Addressed;
use wire_app_shared::Post;

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

    pub fn publish_label(&self) -> &'static str {
        match self {
            Speaking::InReplyTo(_) => "Review & reply",
            Speaking::Freely => "Review & publish",
        }
    }
}
