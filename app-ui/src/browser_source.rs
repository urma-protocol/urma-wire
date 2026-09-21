//! Browser-native Esplora transport. A command sticks to one genesis-checked source.
use bitcoin::{Block, BlockHash, Transaction, Txid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use urma_chain::observation::Chain;
use wasm_bindgen::prelude::*;

pub const CHAIN: Chain = Chain::LitecoinTestnet;
pub fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn js_error(e: JsValue) -> String {
    e.as_string()
        .or_else(|| {
            js_sys::Reflect::get(&e, &JsValue::from_str("message"))
                .ok()
                .and_then(|value| value.as_string())
        })
        .unwrap_or_else(|| "Browser operation failed".into())
}

#[wasm_bindgen(inline_js = r#"
export async function chain_fetch(base, path, post, binary) {
    const url = new URL(base);
    if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash) {
        throw new Error('Chain source must be HTTPS without credentials, query or fragment');
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 20000);
    try {
        const response = await fetch(base.replace(/\/$/, '') + path, {
            method: post ? 'POST' : 'GET', body: post || undefined,
            headers: post ? {'Content-Type': 'text/plain'} : {},
            mode: 'cors', credentials: 'omit', cache: 'no-store', redirect: 'error', signal: controller.signal
        });
        if (response.status === 404) return null;
        const reader = response.body.getReader();
        const chunks = []; let length = 0;
        for (;;) {
            const {done, value} = await reader.read();
            if (done) break;
            length += value.length;
            if (length > 8000000) { await reader.cancel(); throw new Error('Chain response exceeds capacity'); }
            chunks.push(value);
        }
        const bytes = new Uint8Array(length); let offset = 0;
        for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
        if (!response.ok) throw new Error('Chain source HTTP ' + response.status + ': ' + new TextDecoder().decode(bytes).slice(0,512));
        return binary ? Array.from(bytes, b => b.toString(16).padStart(2,'0')).join('') : new TextDecoder().decode(bytes);
    } catch (e) { throw new Error('Chain source unreachable, rejected, or restricted by browser/CORS: ' + e.message); }
    finally { clearTimeout(timeout); }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn chain_fetch(
        base: &str,
        path: &str,
        post: &str,
        binary: bool,
    ) -> Result<JsValue, JsValue>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Presence {
    Missing,
    Mempool,
    Confirmed { height: u64, hash: String },
}

pub struct Source {
    pub url: String,
}
impl Source {
    pub async fn connect(url: String) -> Result<Self, String> {
        let source = Self { url };
        if source.hash(0).await? != CHAIN.genesis().map_err(error)?.0 {
            return Err("Selected source has the wrong Litecoin Testnet genesis".into());
        }
        Ok(source)
    }
    async fn request(
        &self,
        path: &str,
        post: &str,
        binary: bool,
    ) -> Result<Option<String>, String> {
        chain_fetch(&self.url, path, post, binary)
            .await
            .map(|v| v.as_string())
            .map_err(js_error)
    }
    pub async fn text(&self, path: &str) -> Result<String, String> {
        self.request(path, "", false)
            .await?
            .ok_or_else(|| format!("Chain source has no {path}"))
    }
    pub async fn json(&self, path: &str) -> Result<Value, String> {
        serde_json::from_str(&self.text(path).await?).map_err(error)
    }
    pub async fn tip(&self) -> Result<(u64, BlockHash), String> {
        let height = self
            .text("/blocks/tip/height")
            .await?
            .trim()
            .parse::<u64>()
            .map_err(error)?;
        Ok((height, self.hash(height).await?))
    }
    pub async fn hash(&self, height: u64) -> Result<BlockHash, String> {
        self.text(&format!("/block-height/{height}"))
            .await?
            .trim()
            .parse()
            .map_err(error)
    }
    pub async fn block(&self, height: u64) -> Result<Block, String> {
        let expected = self.hash(height).await?;
        let raw = self
            .request(&format!("/block/{expected}/raw"), "", true)
            .await?
            .ok_or("Block not available")?;
        urma_chain::decode::esplora_block(&hex::decode(raw).map_err(error)?, CHAIN, expected)
            .map_err(error)
    }
    pub async fn transaction(&self, txid: Txid) -> Result<Transaction, String> {
        let raw = self.text(&format!("/tx/{txid}/hex")).await?;
        let transaction =
            urma_chain::decode::transaction(&hex::decode(raw.trim()).map_err(error)?, CHAIN)
                .map_err(error)?;
        if transaction.compute_txid() != txid {
            return Err("Source transaction hash mismatch".into());
        }
        Ok(transaction)
    }
    pub async fn presence(&self, txid: Txid) -> Result<Presence, String> {
        let Some(raw) = self
            .request(&format!("/tx/{txid}/status"), "", false)
            .await?
        else {
            return Ok(Presence::Missing);
        };
        let status: Value = serde_json::from_str(&raw).map_err(error)?;
        if status["confirmed"].as_bool() == Some(false) {
            return Ok(Presence::Mempool);
        }
        if status["confirmed"].as_bool() != Some(true) {
            return Err("Invalid transaction status".into());
        }
        let height = status["block_height"]
            .as_u64()
            .ok_or("Missing inclusion height")?;
        let hash = status["block_hash"]
            .as_str()
            .ok_or("Missing inclusion block")?;
        if self.hash(height).await?.to_string() != hash {
            return Ok(Presence::Missing);
        }
        // Verify actual inclusion and complete block integrity, including witness commitment.
        let block = self.block(height).await?;
        if block.block_hash().to_string() != hash
            || !block.txdata.iter().any(|tx| tx.compute_txid() == txid)
        {
            return Err("Source transaction inclusion mismatch".into());
        }
        Ok(Presence::Confirmed {
            height,
            hash: hash.into(),
        })
    }
    pub async fn broadcast(&self, transaction: &Transaction) -> Result<(), String> {
        let raw = hex::encode(bitcoin::consensus::serialize(transaction));
        let result = self
            .request("/tx", &raw, false)
            .await?
            .ok_or("Broadcast endpoint missing")?;
        if result.trim() != transaction.compute_txid().to_string() {
            return Err(
                "Broadcast returned an unexpected transaction ID; resume to reconcile exact bytes"
                    .into(),
            );
        }
        Ok(())
    }
}
