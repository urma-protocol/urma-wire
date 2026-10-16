use crate::browser::chain::{self, Cached};
use crate::browser::funding;
use crate::browser::health;
use crate::browser::mempool;
use crate::browser::publish::{self, Planned};
use crate::browser::session;
use crate::browser::source::Source;
use crate::browser::store::{self, OperationLock};
use crate::config::{retry_delay_secs, SOURCE_KEY, SYNC_RETRY_ATTEMPTS};
use crate::errors::BrowserError;
use crate::projection::{lookup, project, replies};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wire_app_core::connection::{
    safe_endpoint, NodeHealth, SourceChoice, SourceKind, SourceStanding, SourceState,
    TransportStatus,
};
use wire_app_core::contract::{Request, Response};
use wire_app_core::feed::{Feed, FeedSource, Lookup, SyncStatus};
use wire_app_core::model::{Chain, Post, Txid};
use wire_app_core::publication::Pending;

#[wasm_bindgen(inline_js = r#"
export function chain_notice(message) { window.dispatchEvent(new CustomEvent('urma:chain-status', {detail: message})); }
export function sleep_ms(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
"#)]
extern "C" {
    fn chain_notice(message: &str);
    async fn sleep_ms(ms: u32) -> JsValue;
}

pub async fn handle(request: Request) -> Result<Response, BrowserError> {
    match request {
        Request::IdentityStatus => Ok(Response::Identity(session::status().await?)),
        Request::ImportIdentity { vault, password } => {
            Ok(Response::Identity(session::import(vault, password).await?))
        }
        Request::CreateIdentity { phrase, password } => {
            let (status, phrase) = session::create(phrase, password).await?;
            Ok(Response::Created { status, phrase })
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
        | Request::StartSync { chain }
        | Request::SyncStatus { chain }
        | Request::Mempool { chain }
        | Request::Post { chain, .. }
        | Request::Replies { chain, .. }
        | Request::Health { chain }
        | Request::Transport { chain }
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
        Request::SyncStatus { .. } => return Ok(Response::Sync(chain::status())),
        Request::StartSync { .. } => return Ok(Response::Sync(start_sync())),
        Request::Transport { .. } => return Ok(Response::Transport(transport().await?)),
        Request::Feed { .. } => return Ok(Response::Feed(feed().await?)),
        Request::Post { txid, .. } => return Ok(Response::Post(cached_post(*txid).await?)),
        Request::Replies { txid, .. } => {
            return Ok(Response::Replies(cached_replies(*txid).await?))
        }
        Request::Mempool { .. } => {
            return Ok(Response::Mempool(
                mempool::mempool(&connected().await?).await?,
            ))
        }
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
        Request::Wallet { .. } => Ok(Response::Wallet(
            funding::wallet(&connected().await?).await?,
        )),
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

fn start_sync() -> SyncStatus {
    let current = chain::status();
    if !current.settled() {
        return current;
    }
    let begun = chain::begin();
    wasm_bindgen_futures::spawn_local(run_sync());
    begun
}

async fn run_sync() {
    let mut attempt: u32 = 0;
    loop {
        match catch_up().await {
            Ok(()) => return chain::complete(),
            Err(error) => {
                attempt += 1;
                if attempt > SYNC_RETRY_ATTEMPTS {
                    tracing::warn!(error = %error, attempt, "history sync gave up");
                    return chain::fail(format!(
                        "Sync gave up after {SYNC_RETRY_ATTEMPTS} retries: {error}"
                    ));
                }
                tracing::warn!(error = %error, attempt, "history sync failed; retrying");
                let reason = error.to_string();
                for seconds in (1..=retry_delay_secs(attempt)).rev() {
                    chain::retrying(attempt, seconds, &reason);
                    sleep_ms(1000).await;
                }
            }
        }
    }
}

async fn catch_up() -> Result<(), BrowserError> {
    let _lock: OperationLock = OperationLock::acquire().await?;
    let source = connected().await?;
    chain::sync(&source).await
}

async fn transport() -> Result<TransportStatus, BrowserError> {
    let url = store::source().await?;
    Ok(TransportStatus {
        peers: 0,
        sources: vec![SourceState {
            label: safe_endpoint(&url),
            kind: SourceKind::Http,
            standing: SourceStanding::Ready,
        }],
        node: NodeHealth::Unconfigured,
    })
}

async fn feed() -> Result<Feed, BrowserError> {
    let status = chain::status();
    match chain::cached().await? {
        Cached::Index(index) => Ok(project(&index, status.complete_to_tip)?),
        Cached::Empty => Ok(Feed {
            identities: BTreeMap::new(),
            posts: Vec::new(),
            start: status.start,
            tip: status.tip,
            complete_to_tip: false,
            source: FeedSource::AutonomousLocal,
        }),
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
