use crate::browser::chain::{self, Cached};
use crate::browser::funding;
use crate::browser::health;
use crate::browser::publish::{self, Planned};
use crate::browser::session;
use crate::browser::source::Source;
use crate::browser::store::{self, OperationLock};
use crate::config::{SOURCE_KEY, SYNC_BLOCKS};
use crate::errors::BrowserError;
use crate::projection::{lookup, project, replies};
use wasm_bindgen::prelude::wasm_bindgen;
use wire_app_core::connection::SourceChoice;
use wire_app_core::contract::{Request, Response};
use wire_app_core::feed::{Feed, Lookup, SyncProgress};
use wire_app_core::model::{Chain, Post, Txid};
use wire_app_core::publication::Pending;

#[wasm_bindgen(
    inline_js = "export function chain_notice(message) { window.dispatchEvent(new CustomEvent('urma:chain-status', {detail: message})); }"
)]
extern "C" {
    fn chain_notice(message: &str);
}

pub async fn handle(request: Request) -> Result<Response, BrowserError> {
    match request {
        Request::IdentityStatus => Ok(Response::Identity(session::status().await?)),
        Request::ImportIdentity { vault, password } => {
            Ok(Response::Identity(session::import(vault, password).await?))
        }
        Request::Unlock(credential) => {
            session::unlock(credential).await?;
            Ok(Response::Unlocked)
        }
        Request::Lock => {
            session::lock();
            Ok(Response::Locked)
        }
        unserved @ Request::FetchImage { .. } => Err(BrowserError::Unsupported(unserved.name())),
        chained => chain_request(chained).await,
    }
}

fn served(request: &Request) -> Result<(), BrowserError> {
    let chain = match request {
        Request::Wallet { chain }
        | Request::Feed { chain }
        | Request::Sync { chain, .. }
        | Request::Post { chain, .. }
        | Request::Replies { chain, .. }
        | Request::Health { chain }
        | Request::Quote { chain, .. } => *chain,
        _unchained => return Ok(()),
    };
    match chain {
        Chain::Litecoin => Ok(()),
        Chain::Bitcoin => Err(BrowserError::WrongChain),
    }
}

async fn chain_request(request: Request) -> Result<Response, BrowserError> {
    served(&request)?;
    match &request {
        Request::Source => return Ok(Response::Source(health::described(store::source().await?))),
        Request::PublicationStatus => return publication_status().await,
        _locked => (),
    }
    let _lock: OperationLock = OperationLock::acquire().await?;
    match request {
        Request::Health { .. } => Ok(Response::Health(health::health().await?)),
        Request::AdoptSource(SourceChoice::Esplora(url)) => Ok(Response::SourceAdopted(
            health::described(adopt(url).await?),
        )),
        Request::AdoptSource(SourceChoice::Public | SourceChoice::LocalNode) => {
            Err(BrowserError::Unsupported("native chain sources"))
        }
        Request::Post { txid, .. } => Ok(Response::Post(cached_post(txid).await?)),
        Request::Replies { txid, .. } => Ok(Response::Replies(cached_replies(txid).await?)),
        Request::Feed { .. } => Ok(Response::Feed(feed().await?)),
        Request::Sync { max_blocks, .. } => Ok(Response::Sync(slice(max_blocks).await?)),
        Request::Wallet { .. } => Ok(Response::Wallet(funding::wallet(&connected().await?).await?)),
        Request::Quote { content, .. } => Ok(Response::Publication(
            publish::quote(&connected().await?, &content).await?,
        )),
        Request::DiscardQuote { id } => {
            publish::discard(&id).await?;
            Ok(Response::Discarded)
        }
        Request::Advance { id, approve } => Ok(Response::Publication(
            publish::advance(&connected().await?, &id, approve).await?,
        )),
        unserved => Err(BrowserError::Unsupported(unserved.name())),
    }
}

async fn connected() -> Result<Source, BrowserError> {
    Source::connect(store::source().await?).await
}

async fn adopt(url: String) -> Result<String, BrowserError> {
    let url = url.trim().trim_end_matches('/').to_owned();
    Source::connect(url.clone()).await?;
    store::write(SOURCE_KEY, &url).await?;
    Ok(url)
}

async fn cached_post(txid: Txid) -> Result<Lookup, BrowserError> {
    let Cached::Index(index) = chain::cached().await? else {
        return Ok(Lookup::Absent);
    };
    Ok(lookup(&index, txid)?)
}

async fn cached_replies(txid: Txid) -> Result<Vec<Post>, BrowserError> {
    let Cached::Index(index) = chain::cached().await? else {
        return Ok(Vec::new());
    };
    Ok(replies(&index, txid)?)
}

async fn slice(max_blocks: u64) -> Result<SyncProgress, BrowserError> {
    let source = connected().await?;
    Ok(chain::sync(&source, max_blocks).await?.progress)
}

async fn feed() -> Result<Feed, BrowserError> {
    let connection = Source::connect(store::source().await?).await;
    let refreshed = match connection {
        Ok(source) => chain::sync(&source, SYNC_BLOCKS).await,
        Err(error) => Err(error),
    };
    match refreshed {
        Ok(refreshed) => {
            chain_notice("Chain source connected. Confirmed records verified; recent-history catch-up is shown in the feed.");
            Ok(project(&refreshed.index, refreshed.progress.complete_to_tip)?)
        }
        Err(error) => {
            tracing::warn!(error = %error, "feed refresh failed; cached history shown");
            chain_notice(&format!(
                "Showing cached verified history; chain refresh failed: {error}"
            ));
            let Cached::Index(index) = chain::cached().await? else {
                return Err(error);
            };
            Ok(project(&index, false)?)
        }
    }
}

async fn publication_status() -> Result<Response, BrowserError> {
    let Planned::Active(_waiting) = publish::active().await? else {
        return Ok(Response::Pending(Pending::Settled));
    };
    let _lock: OperationLock = OperationLock::acquire().await?;
    let connection = Source::connect(store::source().await?).await;
    let Planned::Active(active) = publish::active().await? else {
        return Err(BrowserError::Vanished);
    };
    let observed = match connection {
        Ok(source) => publish::observe(&source, &active).await,
        Err(error) => Err(error),
    };
    let publication = match observed {
        Ok(publication) => publication,
        Err(error) => {
            tracing::warn!(error = %error, "publication status observed offline");
            chain_notice(&format!(
                "Saved publication; current chain status unavailable: {error}"
            ));
            publish::offline_status(&active)?
        }
    };
    Ok(Response::Pending(Pending::Awaiting(publication)))
}
