use crate::data::engine::ask;
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::Fault;
use wire_app_core::feed::{Feed, Lookup, SyncStatus};
use wire_app_core::model::{Chain, Post, Txid};

pub async fn feed(chain: Chain) -> Result<Feed, Fault> {
    match ask(Request::Feed { chain }).await? {
        Response::Feed(feed) => Ok(feed),
        other => Err(other.unexpected("feed")),
    }
}

pub async fn start_sync(chain: Chain) -> Result<SyncStatus, Fault> {
    match ask(Request::StartSync { chain }).await? {
        Response::Sync(status) => Ok(status),
        other => Err(other.unexpected("history sync start")),
    }
}

pub async fn sync_status(chain: Chain) -> Result<SyncStatus, Fault> {
    match ask(Request::SyncStatus { chain }).await? {
        Response::Sync(status) => Ok(status),
        other => Err(other.unexpected("history sync status")),
    }
}

pub async fn post(chain: Chain, txid: Txid) -> Result<Lookup, Fault> {
    match ask(Request::Post { chain, txid }).await? {
        Response::Post(lookup) => Ok(lookup),
        other => Err(other.unexpected("post")),
    }
}

pub async fn replies(chain: Chain, txid: Txid) -> Result<Vec<Post>, Fault> {
    match ask(Request::Replies { chain, txid }).await? {
        Response::Replies(replies) => Ok(replies),
        other => Err(other.unexpected("replies")),
    }
}
