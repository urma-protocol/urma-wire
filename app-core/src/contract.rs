use crate::connection::ConnectionHealth;
use crate::errors::Fault;
use crate::feed::{Feed, Lookup};
use crate::markdown::FetchedImage;
use crate::model::{Chain, Post, Txid, WalletSummary};
use crate::publication::{Content, Pending, Publication};
use crate::session::{Credential, IdentityStatus};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Request {
    IdentityStatus,
    ImportIdentity { vault: Vec<u8>, password: String },
    Unlock(Credential),
    Lock,
    Wallet { chain: Chain },
    Feed { chain: Chain },
    Post { chain: Chain, txid: Txid },
    Replies { chain: Chain, txid: Txid },
    Health { chain: Chain },
    Source,
    AdoptSource { url: String },
    Quote { chain: Chain, content: Content },
    DiscardQuote { id: String },
    PublicationStatus,
    Advance { id: String, approve: bool },
    FetchImage { url: String },
}

impl Request {
    pub fn name(&self) -> &'static str {
        match self {
            Request::IdentityStatus => "identity status",
            Request::ImportIdentity { .. } => "identity import",
            Request::Unlock(_credential) => "unlock",
            Request::Lock => "lock",
            Request::Wallet { .. } => "wallet",
            Request::Feed { .. } => "feed",
            Request::Post { .. } => "post",
            Request::Replies { .. } => "replies",
            Request::Health { .. } => "connection health",
            Request::Source => "chain source",
            Request::AdoptSource { .. } => "chain source change",
            Request::Quote { .. } => "publication quote",
            Request::DiscardQuote { .. } => "discard publication quote",
            Request::PublicationStatus => "publication status",
            Request::Advance { .. } => "publication advance",
            Request::FetchImage { .. } => "external image",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Identity(IdentityStatus),
    Unlocked,
    Locked,
    Wallet(WalletSummary),
    Feed(Feed),
    Post(Lookup),
    Replies(Vec<Post>),
    Health(ConnectionHealth),
    Source(String),
    SourceAdopted,
    Publication(Publication),
    Discarded,
    Pending(Pending),
    Image(FetchedImage),
}

impl Response {
    pub fn name(&self) -> &'static str {
        match self {
            Response::Identity(_status) => "identity status",
            Response::Unlocked => "unlocked",
            Response::Locked => "locked",
            Response::Wallet(_summary) => "wallet",
            Response::Feed(_feed) => "feed",
            Response::Post(_lookup) => "post",
            Response::Replies(_replies) => "replies",
            Response::Health(_health) => "connection health",
            Response::Source(_url) => "chain source",
            Response::SourceAdopted => "chain source adopted",
            Response::Publication(_publication) => "publication",
            Response::Discarded => "discarded publication quote",
            Response::Pending(_pending) => "pending publication",
            Response::Image(_image) => "external image",
        }
    }

    pub fn unexpected(self, asked: &str) -> Fault {
        Fault::unanswered(asked, self.name())
    }
}
