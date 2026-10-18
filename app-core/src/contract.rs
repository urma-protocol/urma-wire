use crate::connection::{ConnectionHealth, SourceChoice, SourceStatus, TransportStatus};
use crate::errors::Fault;
use crate::feed::{Feed, Lookup, SyncStatus};
use crate::markdown::FetchedImage;
use crate::model::{Chain, Post, Txid, WalletSummary};
use crate::publication::{Content, Pending, Publication};
use crate::session::{Credential, IdentityStatus};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Request {
    IdentityStatus,
    CreateIdentity {
        password: String,
        replace: bool,
    },
    RecoverIdentity {
        phrase: String,
        password: String,
        replace: bool,
    },
    ImportIdentity {
        vault: Vec<u8>,
        password: String,
        replace: bool,
    },
    Unlock(Credential),
    Lock,
    Wallet { chain: Chain },
    Feed { chain: Chain },
    StartSync { chain: Chain },
    SyncStatus { chain: Chain },
    Mempool { chain: Chain },
    Post { chain: Chain, txid: Txid },
    Replies { chain: Chain, txid: Txid },
    Health { chain: Chain },
    Transport { chain: Chain },
    Source,
    AdoptSource(SourceChoice),
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
            Request::CreateIdentity { .. } => "identity creation",
            Request::RecoverIdentity { .. } => "identity recovery",
            Request::ImportIdentity { .. } => "identity import",
            Request::Unlock(_credential) => "unlock",
            Request::Lock => "lock",
            Request::Wallet { .. } => "wallet",
            Request::Feed { .. } => "feed",
            Request::StartSync { .. } => "history sync start",
            Request::SyncStatus { .. } => "history sync status",
            Request::Mempool { .. } => "mempool",
            Request::Post { .. } => "post",
            Request::Replies { .. } => "replies",
            Request::Health { .. } => "connection health",
            Request::Transport { .. } => "transport status",
            Request::Source => "chain source",
            Request::AdoptSource(_choice) => "chain source change",
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
    Created { status: IdentityStatus, phrase: String },
    Wallet(WalletSummary),
    Feed(Feed),
    Sync(SyncStatus),
    Mempool(Vec<Post>),
    Post(Lookup),
    Replies(Vec<Post>),
    Health(ConnectionHealth),
    Transport(TransportStatus),
    Source(SourceStatus),
    SourceAdopted(SourceStatus),
    Publication(Publication),
    Discarded,
    Pending(Pending),
    Image(FetchedImage),
}

impl Response {
    pub fn name(&self) -> &'static str {
        match self {
            Response::Identity(_status) => "identity status",
            Response::Created { .. } => "created identity",
            Response::Wallet(_summary) => "wallet",
            Response::Feed(_feed) => "feed",
            Response::Sync(_status) => "history sync status",
            Response::Mempool(_posts) => "mempool",
            Response::Post(_lookup) => "post",
            Response::Replies(_replies) => "replies",
            Response::Health(_health) => "connection health",
            Response::Transport(_status) => "transport status",
            Response::Source(_status) => "chain source",
            Response::SourceAdopted(_status) => "chain source adopted",
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
