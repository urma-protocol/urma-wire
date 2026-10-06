use crate::data::engine::ask;
use wire_app_core::connection::{ConnectionHealth, SourceChoice, SourceStatus, TransportStatus};
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::Fault;
use wire_app_core::model::{Chain, WalletSummary};

pub async fn wallet(chain: Chain) -> Result<WalletSummary, Fault> {
    match ask(Request::Wallet { chain }).await? {
        Response::Wallet(summary) => Ok(summary),
        other => Err(other.unexpected("wallet")),
    }
}

pub async fn health(chain: Chain) -> Result<ConnectionHealth, Fault> {
    match ask(Request::Health { chain }).await? {
        Response::Health(health) => Ok(health),
        other => Err(other.unexpected("connection health")),
    }
}

pub async fn transport(chain: Chain) -> Result<TransportStatus, Fault> {
    match ask(Request::Transport { chain }).await? {
        Response::Transport(status) => Ok(status),
        other => Err(other.unexpected("transport status")),
    }
}

pub async fn source() -> Result<SourceStatus, Fault> {
    match ask(Request::Source).await? {
        Response::Source(status) => Ok(status),
        other => Err(other.unexpected("chain source")),
    }
}

pub async fn adopt_source(choice: SourceChoice) -> Result<SourceStatus, Fault> {
    match ask(Request::AdoptSource(choice)).await? {
        Response::SourceAdopted(status) => Ok(status),
        other => Err(other.unexpected("chain source change")),
    }
}
