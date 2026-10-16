use crate::errors::IdError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chain {
    Bitcoin,
    Litecoin,
}

impl Chain {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Bitcoin => "btc",
            Self::Litecoin => "ltc",
        }
    }

    pub fn network_name(&self) -> &'static str {
        match self {
            Self::Bitcoin => "Bitcoin Testnet4",
            Self::Litecoin => "Litecoin Testnet",
        }
    }

    pub fn ticker(&self) -> &'static str {
        match self {
            Self::Bitcoin => "BTC",
            Self::Litecoin => "LTC",
        }
    }

    pub fn amount(&self, base_units: u64) -> String {
        format!(
            "{}.{:08} {}",
            base_units / 100_000_000,
            base_units % 100_000_000,
            self.ticker()
        )
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.network_name())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletSummary {
    pub author_pubkey: String,
    pub author_fingerprint: String,
    pub receive_address: String,
    pub chain: Chain,
    pub network_name: String,
    pub confirmed_balance_sats: u64,
    pub unconfirmed_balance_sats: u64,
    pub utxo_count: usize,
    pub tip_height: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorId(#[serde(with = "hex")] pub [u8; 32]);

impl AuthorId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(text: &str) -> Result<Self, IdError> {
        Ok(Self(exact(hex::decode(text)?)?))
    }

    pub fn fingerprint(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl fmt::Debug for AuthorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AuthorId({})", self.fingerprint())
    }
}

impl fmt::Display for AuthorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Txid(#[serde(with = "hex")] pub [u8; 32]);

impl Txid {
    pub const fn from_raw_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_display_hex(&self) -> String {
        let mut reversed = self.0;
        reversed.reverse();
        hex::encode(reversed)
    }

    pub fn from_display_hex(text: &str) -> Result<Self, IdError> {
        let mut bytes = exact(hex::decode(text)?)?;
        bytes.reverse();
        Ok(Self(bytes))
    }

    pub fn short_display(&self) -> String {
        let hex = self.to_display_hex();
        format!("{}...{}", &hex[..8], &hex[hex.len() - 6..])
    }
}

impl fmt::Debug for Txid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Txid({})", self.to_display_hex())
    }
}

impl fmt::Display for Txid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_hex())
    }
}

fn exact(bytes: Vec<u8>) -> Result<[u8; 32], IdError> {
    match <[u8; 32]>::try_from(bytes) {
        Ok(array) => Ok(array),
        Err(rejected) => Err(IdError::Length {
            expected: 32,
            got: rejected.len(),
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedStatus {
    pub block_height: u64,
    pub block_hash: String,
    pub confirmations: u64,
    pub tx_index: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Inclusion {
    Confirmed(ConfirmedStatus),
    Mempool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    Root,
    Reply(Txid),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    pub txid: Txid,
    pub author: AuthorId,
    pub chain: Chain,
    pub text: String,
    pub wire: String,
    pub hashtags: Vec<String>,
    pub placement: Placement,
    pub inclusion: Inclusion,
    pub reply_count: usize,
}
