use crate::config::NETWORK;
use crate::errors::BrowserError;
use bitcoin::{Block, BlockHash, Transaction, Txid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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

enum Answered {
    Body(String),
    NotFound,
}

pub struct Source {
    pub url: String,
}

impl Source {
    pub async fn connect(url: String) -> Result<Self, BrowserError> {
        let source = Self { url };
        if source.hash(0).await? != NETWORK.genesis()?.0 {
            return Err(BrowserError::WrongGenesis);
        }
        Ok(source)
    }

    async fn request(
        &self,
        path: &str,
        post: &str,
        binary: bool,
    ) -> Result<Answered, BrowserError> {
        let answer = chain_fetch(&self.url, path, post, binary).await?;
        let Some(body) = answer.as_string() else {
            return Ok(Answered::NotFound);
        };
        Ok(Answered::Body(body))
    }

    pub async fn text(&self, path: &str) -> Result<String, BrowserError> {
        match self.request(path, "", false).await? {
            Answered::Body(text) => Ok(text),
            Answered::NotFound => Err(BrowserError::Missing(path.to_owned())),
        }
    }

    pub async fn json(&self, path: &str) -> Result<Value, BrowserError> {
        Ok(serde_json::from_str(&self.text(path).await?)?)
    }

    pub async fn tip(&self) -> Result<(u64, BlockHash), BrowserError> {
        let height = self
            .text("/blocks/tip/height")
            .await?
            .trim()
            .parse::<u64>()?;
        Ok((height, self.hash(height).await?))
    }

    pub async fn hash(&self, height: u64) -> Result<BlockHash, BrowserError> {
        Ok(self
            .text(&format!("/block-height/{height}"))
            .await?
            .trim()
            .parse()?)
    }

    pub async fn block(&self, height: u64) -> Result<Block, BrowserError> {
        let expected = self.hash(height).await?;
        let Answered::Body(raw) = self
            .request(&format!("/block/{expected}/raw"), "", true)
            .await?
        else {
            return Err(BrowserError::BlockMissing);
        };
        Ok(urma_chain::decode::esplora_block(
            &hex::decode(raw)?,
            NETWORK,
            expected,
        )?)
    }

    pub async fn transaction(&self, txid: Txid) -> Result<Transaction, BrowserError> {
        let raw = self.text(&format!("/tx/{txid}/hex")).await?;
        let transaction = urma_chain::decode::transaction(&hex::decode(raw.trim())?, NETWORK)?;
        if transaction.compute_txid() != txid {
            return Err(BrowserError::TransactionMismatch);
        }
        Ok(transaction)
    }

    pub async fn presence(&self, txid: Txid) -> Result<Presence, BrowserError> {
        let Answered::Body(raw) = self.request(&format!("/tx/{txid}"), "", false).await? else {
            return Ok(Presence::Missing);
        };
        let transaction: Value = serde_json::from_str(&raw)?;
        let status = &transaction["status"];
        match status["confirmed"].as_bool() {
            Some(true) => (),
            Some(false) => return Ok(Presence::Mempool),
            None => return Err(BrowserError::InvalidStatus),
        }
        let Some(height) = status["block_height"].as_u64() else {
            return Err(BrowserError::InclusionMissing("height"));
        };
        let Some(hash) = status["block_hash"].as_str() else {
            return Err(BrowserError::InclusionMissing("block"));
        };
        if self.hash(height).await?.to_string() != hash {
            return Ok(Presence::Missing);
        }
        let block = self.block(height).await?;
        if block.block_hash().to_string() != hash
            || !block.txdata.iter().any(|tx| tx.compute_txid() == txid)
        {
            return Err(BrowserError::InclusionMismatch);
        }
        Ok(Presence::Confirmed {
            height,
            hash: hash.into(),
        })
    }

    pub async fn broadcast(&self, transaction: &Transaction) -> Result<(), BrowserError> {
        let raw = hex::encode(bitcoin::consensus::serialize(transaction));
        let Answered::Body(result) = self.request("/tx", &raw, false).await? else {
            return Err(BrowserError::BroadcastMissing);
        };
        if result.trim() != transaction.compute_txid().to_string() {
            return Err(BrowserError::BroadcastMismatch);
        }
        Ok(())
    }
}
