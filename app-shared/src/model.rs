use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Supported isolated blockchain contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Chain {
    #[default]
    Bitcoin,
    Litecoin,
}

impl Chain {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bitcoin => "Bitcoin",
            Self::Litecoin => "Litecoin",
        }
    }

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
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.network_name())
    }
}

/// Wallet and identity summary presented to the user.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
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

/// Publication fee review quote presented prior to on-chain broadcasting.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicationQuote {
    pub content_bytes: usize,
    pub max_bytes: usize,
    pub chain: Chain,
    pub network_name: String,
    pub fee_rate: u64,
    pub estimated_fee_sats: u64,
    pub spendable_balance_sats: u64,
    pub sufficient_balance: bool,
    pub warning: Option<String>,
}

/// Spendable unspent transaction output.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpendableUtxo {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub confirmations: u64,
}

/// URMA author identity: a 32-byte BIP 340 x-only public key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    pub fn from_hex(s: &str) -> Result<Self, WireError> {
        let bytes = hex::decode(s).map_err(|_| WireError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(WireError::InvalidLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Key fingerprint: first 8 hex characters for unambiguous identification.
    pub fn fingerprint(&self) -> String {
        hex::encode(&self.0[..4])
    }

    /// Short display format: "f7a1...8c4d"
    pub fn short_display(&self) -> String {
        let full = self.to_hex();
        format!("{}...{}", &full[..6], &full[full.len() - 4..])
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

/// 32-byte transaction identifier.
/// Wire order is used on chain / inside reply target (reversed relative to displayed RPC hex).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Txid(#[serde(with = "hex")] pub [u8; 32]);

impl Txid {
    pub const fn from_raw_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_raw_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Display hex (Bitcoin RPC convention: bytes reversed).
    pub fn to_display_hex(&self) -> String {
        let mut rev = self.0;
        rev.reverse();
        hex::encode(rev)
    }

    /// Parse from Bitcoin display hex string.
    pub fn from_display_hex(s: &str) -> Result<Self, WireError> {
        let mut bytes = hex::decode(s).map_err(|_| WireError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(WireError::InvalidLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        bytes.reverse();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Raw wire hex (preserves array byte order).
    pub fn to_wire_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Short representation for UI display.
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

/// URMA V0 public record kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RecordKind {
    Post = 2,
    Reply = 4,
    Profile = 5,
    Avatar = 6,
    WirePost = 10,
    WireReply = 11,
}

impl RecordKind {
    pub fn from_u8(byte: u8) -> Result<Self, WireError> {
        match byte {
            2 => Ok(Self::Post),
            4 => Ok(Self::Reply),
            5 => Ok(Self::Profile),
            6 => Ok(Self::Avatar),
            10 => Ok(Self::WirePost),
            11 => Ok(Self::WireReply),
            other => Err(WireError::UnsupportedKind(other)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Confirmation and chain location status.
/// In accordance with D-017, mempool (0 confirmations) is strictly ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedStatus {
    pub block_height: u64,
    pub block_hash: String,
    pub confirmations: u64,
    pub tx_index: u32,
}

impl ConfirmedStatus {
    pub fn is_confirmed(&self) -> bool {
        self.confirmations >= 1
    }
}

/// A confirmed post or reply projected into the feed, including structured metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    pub txid: Txid,
    pub author: AuthorId,
    pub chain: Chain,
    pub text: String,
    #[serde(default)]
    pub wire: String,
    #[serde(default)]
    pub hashtags: Vec<String>,
    pub status: ConfirmedStatus,
    pub reply_count: usize,
}

/// A confirmed public reply (URMA Kind 04).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub txid: Txid,
    pub author: AuthorId,
    pub chain: Chain,
    pub target_txid: Txid,
    pub text: String,
    pub status: ConfirmedStatus,
}

/// A confirmed author profile pseudonym (URMA Kind 05).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub author: AuthorId,
    pub chain: Chain,
    pub pseudonym: String,
    pub status: ConfirmedStatus,
}

/// A confirmed pixel-art avatar (URMA Kind 06, exactly 512 raw bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarRecord {
    pub author: AuthorId,
    pub chain: Chain,
    #[serde(with = "avatar_bytes")]
    pub pixels: [u8; 512],
    pub status: ConfirmedStatus,
}

mod avatar_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 512], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 512], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let vec = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if vec.len() != 512 {
            return Err(serde::de::Error::custom("avatar must be exactly 512 bytes"));
        }
        let mut arr = [0u8; 512];
        arr.copy_from_slice(&vec);
        Ok(arr)
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    #[error("hex parsing error")]
    InvalidHex,
    #[error("invalid length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },
    #[error("record truncated: length {0} is less than URMA 8-byte prefix")]
    TruncatedPrefix(usize),
    #[error("invalid marker: expected 'URMA'")]
    InvalidMarker,
    #[error("unsupported URMA version: {0}")]
    UnsupportedVersion(u8),
    #[error("reserved flags must be zero in V0")]
    NonzeroFlags,
    #[error("unsupported record kind: {0}")]
    UnsupportedKind(u8),
    #[error("record exceeds 32 KiB cap: {0} bytes")]
    ExceedsCapacity(usize),
    #[error("invalid UTF-8 payload")]
    InvalidUtf8,
    #[error("invalid avatar payload length: expected exactly 512 bytes, got {0}")]
    InvalidAvatarSize(usize),
    #[error("truncated reply target: requires 32 bytes")]
    TruncatedReplyTarget,
    #[error("mempool unconfirmed transactions are ignored (D-017)")]
    UnconfirmedMempool,
    #[error("target record not found: {0}")]
    NotFound(String),
    #[error("chain mismatch: cross-chain actions are forbidden")]
    ChainMismatch,
    #[error("reorg rollback failed: {0}")]
    ReorgFailed(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<urma_runtime::error::Error> for WireError {
    fn from(err: urma_runtime::error::Error) -> Self {
        Self::Protocol(err.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<urma_io::Error> for WireError {
    fn from(error: urma_io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<std::io::Error> for WireError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<bitcoin::Txid> for Txid {
    fn from(t: bitcoin::Txid) -> Self {
        use bitcoin::hashes::Hash;
        Self(t.to_byte_array())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<Txid> for bitcoin::Txid {
    fn from(t: Txid) -> Self {
        use bitcoin::hashes::Hash;
        bitcoin::Txid::from_byte_array(t.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<bitcoin::XOnlyPublicKey> for AuthorId {
    fn from(k: bitcoin::XOnlyPublicKey) -> Self {
        Self(k.serialize())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl TryFrom<AuthorId> for bitcoin::XOnlyPublicKey {
    type Error = WireError;
    fn try_from(a: AuthorId) -> Result<Self, Self::Error> {
        bitcoin::XOnlyPublicKey::from_slice(&a.0)
            .map_err(|e| WireError::Protocol(format!("invalid x-only public key: {e}")))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<urma_core::error::Error> for WireError {
    fn from(err: urma_core::error::Error) -> Self {
        Self::Protocol(err.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<urma_identity::error::IdentityError> for WireError {
    fn from(error: urma_identity::error::IdentityError) -> Self {
        Self::Protocol(error.to_string())
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl From<urma_wallet::wallet::WalletError> for WireError {
    fn from(error: urma_wallet::wallet::WalletError) -> Self {
        Self::Protocol(error.to_string())
    }
}
