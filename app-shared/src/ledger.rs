#[cfg(not(target_arch = "wasm32"))]
use serde::Deserialize;

use crate::model::{Chain, PublicationQuote, SpendableUtxo, WireError};

pub const BITCOIN_TESTNET4_ESPLORA: &str = "https://mempool.space/testnet4/api";
pub const LITECOIN_TESTNET_ESPLORA: &str = "https://litecoinspace.org/testnet/api";
pub const LITECOIN_CORE_TESTNET_RPC: &str = "http://127.0.0.1:19332";

#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct EsploraStats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct EsploraAddressResponse {
    chain_stats: EsploraStats,
    mempool_stats: EsploraStats,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct EsploraUtxoStatus {
    confirmed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct EsploraUtxo {
    txid: String,
    vout: u32,
    value: u64,
    status: EsploraUtxoStatus,
}

/// Blockchain network ledger client for balance queries, UTXO lookups, and fee estimations.
#[cfg(not(target_arch = "wasm32"))]
pub struct LedgerClient;

#[cfg(not(target_arch = "wasm32"))]
impl LedgerClient {
    /// Returns the active API base URL for the selected chain context (strictly testnet).
    pub fn api_base(chain: Chain) -> &'static str {
        match chain {
            Chain::Bitcoin => BITCOIN_TESTNET4_ESPLORA,
            Chain::Litecoin => LITECOIN_TESTNET_ESPLORA,
        }
    }

    /// Queries confirmed and unconfirmed balance for an address.
    pub fn get_balance(chain: Chain, address: &str) -> Result<(u64, u64), WireError> {
        let base = Self::api_base(chain);
        let url = format!("{base}/address/{address}");
        let resp = minreq::get(&url)
            .with_timeout(10)
            .send()
            .map_err(|e| WireError::Io(format!("network request failed: {e}")))?;

        if resp.status_code == 200 {
            let data: EsploraAddressResponse = serde_json::from_slice(resp.as_bytes())
                .map_err(|e| WireError::Io(format!("failed to parse address JSON: {e}")))?;
            let confirmed = data
                .chain_stats
                .funded_txo_sum
                .saturating_sub(data.chain_stats.spent_txo_sum);
            let unconfirmed = data
                .mempool_stats
                .funded_txo_sum
                .saturating_sub(data.mempool_stats.spent_txo_sum);
            Ok((confirmed, unconfirmed))
        } else if resp.status_code == 404 {
            // Unused fresh address has 0 balance
            Ok((0, 0))
        } else {
            Err(WireError::Io(format!(
                "endpoint returned HTTP {}",
                resp.status_code
            )))
        }
    }

    /// Queries spendable confirmed UTXOs for an address.
    pub fn get_utxos(chain: Chain, address: &str) -> Result<Vec<SpendableUtxo>, WireError> {
        let base = Self::api_base(chain);
        let url = format!("{base}/address/{address}/utxo");
        let resp = minreq::get(&url)
            .with_timeout(10)
            .send()
            .map_err(|e| WireError::Io(format!("network request failed: {e}")))?;

        if resp.status_code == 200 {
            let raw: Vec<EsploraUtxo> = serde_json::from_slice(resp.as_bytes())
                .map_err(|e| WireError::Io(format!("failed to parse UTXO JSON: {e}")))?;
            let list = raw
                .into_iter()
                .filter(|u| u.status.confirmed)
                .map(|u| SpendableUtxo {
                    txid: u.txid,
                    vout: u.vout,
                    amount_sats: u.value,
                    confirmations: 1,
                })
                .collect();
            Ok(list)
        } else if resp.status_code == 404 {
            Ok(Vec::new())
        } else {
            Err(WireError::Io(format!(
                "endpoint returned HTTP {}",
                resp.status_code
            )))
        }
    }

    /// Queries the latest block tip height.
    pub fn get_tip_height(chain: Chain) -> Result<u64, WireError> {
        let base = Self::api_base(chain);
        let url = format!("{base}/blocks/tip/height");
        let resp = minreq::get(&url)
            .with_timeout(10)
            .send()
            .map_err(|e| WireError::Io(format!("network request failed: {e}")))?;

        if resp.status_code == 200 {
            let text = resp.as_str().map_err(|e| WireError::Io(e.to_string()))?;
            text.trim()
                .parse::<u64>()
                .map_err(|e| WireError::Io(e.to_string()))
        } else {
            Err(WireError::Io(format!(
                "endpoint returned HTTP {}",
                resp.status_code
            )))
        }
    }

    /// Computes a fee quote and review document prior to on-chain publication.
    pub fn quote_publication(
        chain: Chain,
        text: &str,
        fee_rate: u64,
        spendable_balance: u64,
        signer: &impl urma_identity::identity::IdentitySigner,
    ) -> Result<PublicationQuote, WireError> {
        use urma_chain::observation::Chain as ProtocolChain;
        use urma_core::format::{PublicRecord, Urma};

        let content_bytes = text.len();
        let max_bytes = Urma::MAX_PUBLIC_BYTES - Urma::PREFIX_BYTES;
        let record = PublicRecord::Post(text.to_owned()).encode()?;
        let script = urma_wallet::signing::script(signer)?;
        let author = signer.public_key().inner.x_only_public_key().0;
        let estimated_fee_sats =
            urma_runtime::publication::quote_record(&record, author, &script, fee_rate)?;
        let protocol_chain = match chain {
            Chain::Bitcoin => ProtocolChain::BitcoinTestnet4,
            Chain::Litecoin => ProtocolChain::LitecoinTestnet,
        };
        let retained = urma_runtime::config::publication_return(protocol_chain);
        let required = retained
            .checked_mul(2)
            .and_then(|value| value.checked_add(estimated_fee_sats))
            .ok_or_else(|| WireError::Protocol("publication amount overflow".into()))?;
        let sufficient_balance = spendable_balance >= required;
        let warning = if !sufficient_balance {
            Some(format!(
                "Insufficient confirmed balance: need {required} base units including fees and retained outputs, but wallet holds {spendable_balance}"
            ))
        } else {
            None
        };

        Ok(PublicationQuote {
            content_bytes,
            max_bytes,
            chain,
            network_name: chain.network_name().to_string(),
            fee_rate,
            estimated_fee_sats,
            spendable_balance_sats: spendable_balance,
            sufficient_balance,
            warning,
        })
    }
}
