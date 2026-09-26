use crate::browser::chain::{self, Cached};
use crate::browser::source::Source;
use crate::browser::store;
use crate::config::DEFAULT_SOURCE;
use crate::errors::BrowserError;
use wasm_bindgen::prelude::wasm_bindgen;
use wire_app_core::connection::{safe_endpoint, ConnectionHealth, Trouble};
use wire_app_core::model::Chain;

#[wasm_bindgen(inline_js = "export function now_ms() { return BigInt(Date.now()); }")]
extern "C" {
    fn now_ms() -> u64;
}

pub async fn health() -> Result<ConnectionHealth, BrowserError> {
    let url = store::source().await?;
    let mut health = ConnectionHealth {
        network: Chain::Litecoin.network_name().into(),
        source_type: source_type(&url).into(),
        endpoint: safe_endpoint(&url),
        checked_at_ms: now_ms(),
        remote_height: None,
        local_start: None,
        local_height: None,
        trouble: Trouble::Clear,
    };
    match chain::cached().await {
        Ok(Cached::Index(index)) => {
            health.local_start = Some(index.start);
            health.local_height = index.blocks.last().map(|block| block.height);
        }
        Ok(Cached::Empty) => (),
        Err(error) => {
            tracing::warn!(error = %error, "cached browser index could not be read");
            health.trouble = Trouble::IndexUnreadable("Local index unavailable".into());
        }
    }
    match remote_tip(url).await {
        Ok(height) => health.remote_height = Some(height),
        Err(error) => {
            tracing::warn!(error = %error, "browser chain source check failed");
            health.trouble = Trouble::SourceUnverified(
                "Chain source unavailable, rejected, or blocked by browser/CORS".into(),
            );
        }
    }
    Ok(health)
}

async fn remote_tip(url: String) -> Result<u64, BrowserError> {
    Ok(Source::connect(url).await?.tip().await?.0)
}

fn source_type(url: &str) -> &'static str {
    match url == DEFAULT_SOURCE {
        true => "Default public Esplora API",
        false => "Configured browser Esplora API",
    }
}
