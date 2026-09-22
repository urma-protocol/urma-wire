use crate::data::engine::ask;
use crate::data::failure::Failure;
use serde_json::json;
use wire_app_shared::connection::ConnectionHealth;
use wire_app_shared::Chain;
use wire_app_shared::WalletSummary;

pub async fn wallet(chain: Chain) -> Result<WalletSummary, Failure> {
    ask("get_wallet_summary", &json!({ "chain": chain.code() })).await
}

pub async fn health(chain: Chain) -> Result<ConnectionHealth, Failure> {
    ask("get_connection_health", &json!({ "chain": chain.code() })).await
}

pub async fn source() -> Result<String, Failure> {
    ask("get_chain_source", &json!({})).await
}

pub async fn adopt_source(url: String) -> Result<(), Failure> {
    ask("set_chain_source", &json!({ "url": url })).await
}
