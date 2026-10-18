use thiserror::Error;
use wire_app_core::errors::{Fault, FaultKind, IdError};

#[derive(Debug, Error)]
pub enum RecordError {
    #[error(transparent)]
    Protocol(#[from] urma_core::error::Error),
    #[error("Write a post or reply first")]
    EmptyText,
    #[error("An avatar must contain exactly 128 packed pixel bytes (16 × 16)")]
    AvatarSize,
    #[error("kind {0:02X} is not a Wire record")]
    NotWire(u8),
}

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("Indexed record has no block checkpoint")]
    NoCheckpoint,
    #[error("Invalid index height")]
    HeightAboveTip,
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error(transparent)]
    Record(#[from] urma_wire::Error),
    #[error(transparent)]
    Id(#[from] IdError),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Error)]
pub enum MediaError {
    #[error(transparent)]
    Url(#[from] wire_app_core::errors::ImageUrlError),
    #[error("Missing image host")]
    NoHost,
    #[error("Missing image port")]
    NoPort,
    #[error("Image host resolution failed")]
    Resolution(#[source] std::io::Error),
    #[error("Image destination is not public unicast")]
    NotPublic,
    #[error("Cannot initialize checked image transport")]
    Transport(#[source] reqwest::Error),
    #[error("Image request failed")]
    Request(#[source] reqwest::Error),
    #[error("Image exceeds one redirect")]
    Redirects,
    #[error("Invalid image redirect")]
    Redirect,
    #[error("Invalid image redirect")]
    RedirectTarget(#[source] url::ParseError),
    #[error("Image host returned an unsuccessful response")]
    Status,
    #[error("Image download exceeds 4 MiB")]
    TooLarge,
    #[error("Image download exceeds 4 MiB")]
    Oversized(#[from] std::num::TryFromIntError),
    #[error("Image response has no MIME type")]
    NoMime,
    #[error("Image download failed")]
    Download(#[source] std::io::Error),
    #[error("Image response is not an image type")]
    NotImage,
    #[error("Unrecognized image signature")]
    Signature(#[source] image::ImageError),
    #[error("Unsupported image format")]
    Format,
    #[error("Invalid image dimensions")]
    Dimensions(#[source] image::ImageError),
    #[error("Image exceeds 4096-pixel dimension or 4-megapixel bounds")]
    Bounds,
    #[error("Image failed bounded decoding")]
    Decoding(#[source] image::ImageError),
    #[error("Image rendering failed")]
    Rendering(#[source] image::ImageError),
    #[error("Rendered image exceeds bounds")]
    Rendered,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Error)]
pub enum NodeProbeError {
    #[error("not installed (no {})", .0.display())]
    NotInstalled(std::path::PathBuf),
    #[error("no access to {}: add your user to group urma-node and log in again", .0.display())]
    NoAccess(std::path::PathBuf),
    #[error("token {} refused: {cause}", .path.display())]
    Token {
        path: std::path::PathBuf,
        #[source]
        cause: Box<NativeError>,
    },
    #[error("not running")]
    NotRunning,
    #[error("not answering")]
    NotAnswering,
    #[error("unreachable: {0}")]
    Unreachable(std::io::ErrorKind),
    #[error("invalid status: {0:?}")]
    InvalidStatus(serde_json::error::Category),
    #[error("refused the request; {0} byte diagnostic withheld")]
    Refused(usize),
    #[error("busy; {0} byte diagnostic withheld")]
    Busy(usize),
    #[error("serves another network or scan rules")]
    ForeignArchive,
    #[error(transparent)]
    Genesis(#[from] bitcoin::hex::HexToArrayError),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Error)]
pub enum NativeError {
    #[error("Optional URMA archive: {0}")]
    Archive(String),
    #[error("Mempool listing: {0}")]
    Mempool(&'static str),
    #[error("HOME is not set")]
    NoHome,
    #[error("ProgramData is not set")]
    NoProgramData,
    #[error("RPC URL must be UTF-8")]
    RpcUrlEncoding(std::ffi::OsString),
    #[error("Local node is selected but rpc_url and node_auth_file are not configured. Choose Public sources in Settings or configure the node.")]
    LocalNodeUnconfigured,
    #[error("Local node cookie not found at {}. Start litecoind or switch the chain source to Public sources in Settings.", .0.display())]
    CookieMissing(std::path::PathBuf),
    #[error("This Wire V0 flow supports Litecoin Testnet only")]
    WrongChain,
    #[error("The desktop engine does not serve {0}")]
    Unsupported(&'static str),
    #[error("Wire is already running with this data directory")]
    AlreadyRunning(#[source] std::io::Error),
    #[error("Missing unlock file")]
    NoUnlockFile,
    #[error("Import an encrypted URMA vault first")]
    NoVault,
    #[error("Encrypted URMA vault exceeds the size limit")]
    VaultTooLarge,
    #[error("An identity is already stored on this device. Confirm replacing it first.")]
    LocalVaultExists,
    #[error("Cannot decrypt this URMA vault with the supplied password")]
    WrongPassword(#[source] urma_identity::error::IdentityError),
    #[error("Identity vault is locked. Unlock it in Identity.")]
    Locked,
    #[error("Unlock your identity in Identity first")]
    SignerLocked,
    #[error("Balance overflow")]
    BalanceOverflow,
    #[error("Invalid active plan ID")]
    InvalidPlanId,
    #[error("Saved publication cannot be read or validated: {0}")]
    SavedPlan(#[source] urma_runtime::error::Error),
    #[error("Active publication does not match its Litecoin Testnet plan")]
    ForeignPlan,
    #[error("A publication is still being sent. Wait for it, or abandon it from its panel if it never reached the network.")]
    Unfinished,
    #[error("The previous publication is waiting for a Litecoin block. Publish again after it confirms.")]
    Confirming,
    #[error("Confirmed change could not be remembered: {0}")]
    Change(&'static str),
    #[error("No prepared publication")]
    NoPublication,
    #[error("Approval does not match the current exact plan")]
    PlanMismatch,
    #[error("Exact plan approval required before broadcast")]
    ApprovalRequired,
    #[error(
        "{0}. Publication paused; approved fees unchanged. Resume explicitly after resolving it."
    )]
    Paused(String),
    #[error("Chain changed during observation; refresh or resume for fresh evidence")]
    ChainChanged,
    #[error("Invalid plan fee")]
    InvalidPlanFee,
    #[error(transparent)]
    Clock(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    Width(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Files(#[from] urma_io::Error),
    #[error(transparent)]
    Runtime(#[from] urma_runtime::error::Error),
    #[error(transparent)]
    Index(#[from] urma_wire::Error),
    #[error(transparent)]
    Sync(#[from] urma_wire::SyncError<urma_runtime::error::Error>),
    #[error(transparent)]
    Identity(#[from] urma_identity::error::IdentityError),
    #[error(transparent)]
    Wallet(#[from] urma_wallet::wallet::WalletError),
    #[error(transparent)]
    Protocol(#[from] urma_core::error::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error(transparent)]
    Genesis(#[from] bitcoin::hex::HexToArrayError),
    #[error(transparent)]
    Consensus(#[from] bitcoin::consensus::encode::Error),
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Media(#[from] MediaError),
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeError {
    fn kind(&self) -> FaultKind {
        match self {
            Self::WrongChain | Self::Record(_) => FaultKind::Refused,
            Self::Unsupported(_) => FaultKind::Unsupported,
            Self::NoUnlockFile
            | Self::NoVault
            | Self::VaultTooLarge
            | Self::LocalVaultExists
            | Self::WrongPassword(_)
            | Self::Locked
            | Self::SignerLocked
            | Self::Identity(_)
            | Self::Wallet(_) => FaultKind::Identity,
            Self::BalanceOverflow
            | Self::InvalidPlanId
            | Self::ForeignPlan
            | Self::SavedPlan(_)
            | Self::Unfinished
            | Self::Confirming
            | Self::Change(_)
            | Self::NoPublication
            | Self::PlanMismatch
            | Self::ApprovalRequired
            | Self::Paused(_)
            | Self::InvalidPlanFee
            | Self::Protocol(_)
            | Self::Hex(_)
            | Self::Consensus(_) => FaultKind::Publication,
            Self::Archive(_)
            | Self::Mempool(_)
            | Self::ChainChanged
            | Self::Runtime(_)
            | Self::Sync(_)
            | Self::Genesis(_) => FaultKind::Source,
            Self::NoHome
            | Self::NoProgramData
            | Self::RpcUrlEncoding(_)
            | Self::LocalNodeUnconfigured
            | Self::CookieMissing(_) => FaultKind::Source,
            Self::AlreadyRunning(_)
            | Self::Clock(_)
            | Self::Width(_)
            | Self::Io(_)
            | Self::Files(_)
            | Self::Index(_)
            | Self::Json(_)
            | Self::Projection(_) => FaultKind::Storage,
            Self::Media(_) => FaultKind::Media,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<NativeError> for Fault {
    fn from(error: NativeError) -> Self {
        Fault::new(error.kind(), error.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("{0}")]
    Js(String),
    #[error("Browser operation failed")]
    Unreadable,
    #[error("Only Litecoin Testnet is enabled")]
    WrongChain,
    #[error("The browser engine does not serve {0}")]
    Unsupported(&'static str),
    #[error("Browser identities require a password each session")]
    PasswordRequired,
    #[error("Import an encrypted URMA vault first")]
    NoVault,
    #[error("Vault unlock failed; check the password and URMA vault file")]
    Unlock(#[source] urma_identity::error::IdentityError),
    #[error("Encrypted browser storage failed; the previous vault stands")]
    VaultStorage(String),
    #[error("An identity is already stored in this browser. Confirm replacing it first.")]
    LocalVaultExists,
    #[error("Invalid stored vault")]
    StoredVault(#[source] serde_json::Error),
    #[error("Identity operation cancelled by a newer lock or unlock")]
    Cancelled,
    #[error("Identity changed or locked while preparing; review again")]
    SignerChanged,
    #[error("Unlock your identity on the Identity page first")]
    Locked,
    #[error("Invalid browser chain index")]
    InvalidIndex,
    #[error("Noncontiguous browser checkpoints")]
    Noncontiguous,
    #[error("Index height overflow")]
    HeightOverflow,
    #[error("Chain changed while indexing; refresh to roll back and continue")]
    ReorgWhileIndexing,
    #[error("Chain changed during verification; refresh to continue")]
    ChangedDuringVerification,
    #[error("Local chain index capacity reached")]
    IndexCapacity,
    #[error("Browser state exceeds capacity")]
    StateCapacity,
    #[error("Selected source has the wrong Litecoin Testnet genesis")]
    WrongGenesis,
    #[error("Chain source has no {0}")]
    Missing(String),
    #[error("Block not available")]
    BlockMissing,
    #[error("Source transaction hash mismatch")]
    TransactionMismatch,
    #[error("Invalid transaction status")]
    InvalidStatus,
    #[error("Missing inclusion {0}")]
    InclusionMissing(&'static str),
    #[error("Source transaction inclusion mismatch")]
    InclusionMismatch,
    #[error("Broadcast endpoint missing")]
    BroadcastMissing,
    #[error("Broadcast returned an unexpected transaction ID; resume to reconcile exact bytes")]
    BroadcastMismatch,
    #[error("Invalid UTXO response")]
    UtxoResponse,
    #[error("Source UTXO capacity exceeded")]
    UtxoCapacity,
    #[error("UTXO missing {0}")]
    UtxoField(&'static str),
    #[error("Source returned duplicate funding outputs")]
    DuplicateFunding,
    #[error("UTXO output absent")]
    UtxoAbsent,
    #[error("Funding amount or script disagrees with identity transaction")]
    FundingMismatch,
    #[error("Chain changed during funding verification")]
    FundingReorg,
    #[error("Chain changed during funding verification; retry")]
    FundingChanged,
    #[error("Invalid funding spend status")]
    SpendStatus,
    #[error("Balance overflow")]
    BalanceOverflow,
    #[error("Finish the approved publication before preparing another: resume it from your publication panel")]
    Unfinished,
    #[error("Funding budget overflow")]
    BudgetOverflow,
    #[error("No single confirmed spendable output covers the exact fee and return outputs")]
    NoFundingOutput,
    #[error("Active plan must contain one Litecoin Testnet record")]
    ForeignPlan,
    #[error("Stored approval does not match the exact signed plan")]
    ApprovalMismatch,
    #[error("Chain changed during publication observation; resume")]
    ObservationChanged,
    #[error("Invalid plan fee")]
    InvalidPlanFee,
    #[error("No prepared publication")]
    NoPublication,
    #[error("No publication")]
    Vanished,
    #[error("Approval does not match the current exact plan")]
    PlanMismatch,
    #[error("Exact plan approval required before broadcast")]
    ApprovalRequired,
    #[error("Transaction missing after submission; resume the exact approved plan")]
    SubmissionMissing,
    #[error("Chain changed during publication reconciliation; resume exact plan")]
    ReconciliationChanged,
    #[error("Unknown chain fixture command")]
    FixtureCommand,
    #[error("Chain fixture could not compute its {0}")]
    Fixture(&'static str),
    #[error(transparent)]
    Signature(#[from] bitcoin::secp256k1::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error(transparent)]
    Chain(#[from] urma_chain::decode::DecodeError),
    #[error(transparent)]
    Consensus(#[from] bitcoin::consensus::encode::Error),
    #[error(transparent)]
    Hash(#[from] bitcoin::hex::HexToArrayError),
    #[error(transparent)]
    Integer(#[from] std::num::ParseIntError),
    #[error(transparent)]
    Width(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Protocol(#[from] urma_core::error::Error),
    #[error(transparent)]
    Runtime(#[from] urma_runtime::error::Error),
    #[error(transparent)]
    Identity(#[from] urma_identity::error::IdentityError),
    #[error(transparent)]
    Wallet(#[from] urma_wallet::wallet::WalletError),
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Record(#[from] RecordError),
}

#[cfg(target_arch = "wasm32")]
impl BrowserError {
    fn kind(&self) -> FaultKind {
        match self {
            Self::WrongChain | Self::Record(_) => FaultKind::Refused,
            Self::Unsupported(_) => FaultKind::Unsupported,
            Self::PasswordRequired
            | Self::NoVault
            | Self::LocalVaultExists
            | Self::Unlock(_)
            | Self::Cancelled
            | Self::SignerChanged
            | Self::Locked
            | Self::Identity(_)
            | Self::Wallet(_) => FaultKind::Identity,
            Self::VaultStorage(_)
            | Self::StoredVault(_)
            | Self::InvalidIndex
            | Self::Noncontiguous
            | Self::HeightOverflow
            | Self::IndexCapacity
            | Self::StateCapacity
            | Self::Json(_)
            | Self::Projection(_) => FaultKind::Storage,
            Self::Js(_)
            | Self::Unreadable
            | Self::ReorgWhileIndexing
            | Self::ChangedDuringVerification
            | Self::WrongGenesis
            | Self::Missing(_)
            | Self::BlockMissing
            | Self::TransactionMismatch
            | Self::InvalidStatus
            | Self::InclusionMissing(_)
            | Self::InclusionMismatch
            | Self::UtxoResponse
            | Self::UtxoCapacity
            | Self::UtxoField(_)
            | Self::DuplicateFunding
            | Self::UtxoAbsent
            | Self::FundingMismatch
            | Self::FundingReorg
            | Self::FundingChanged
            | Self::SpendStatus
            | Self::Chain(_)
            | Self::Hash(_)
            | Self::Integer(_)
            | Self::Runtime(_) => FaultKind::Source,
            Self::BroadcastMissing
            | Self::BroadcastMismatch
            | Self::BalanceOverflow
            | Self::Unfinished
            | Self::BudgetOverflow
            | Self::NoFundingOutput
            | Self::ForeignPlan
            | Self::ApprovalMismatch
            | Self::ObservationChanged
            | Self::InvalidPlanFee
            | Self::NoPublication
            | Self::Vanished
            | Self::PlanMismatch
            | Self::ApprovalRequired
            | Self::SubmissionMissing
            | Self::ReconciliationChanged
            | Self::Hex(_)
            | Self::Consensus(_)
            | Self::Width(_)
            | Self::Protocol(_) => FaultKind::Publication,
            Self::FixtureCommand | Self::Fixture(_) => FaultKind::Unsupported,
            Self::Signature(_) => FaultKind::Identity,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl From<BrowserError> for Fault {
    fn from(error: BrowserError) -> Self {
        Fault::new(error.kind(), error.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for BrowserError {
    fn from(thrown: wasm_bindgen::JsValue) -> Self {
        let Some(text) = thrown.as_string() else {
            return described(&thrown);
        };
        BrowserError::Js(text)
    }
}

#[cfg(target_arch = "wasm32")]
fn described(thrown: &wasm_bindgen::JsValue) -> BrowserError {
    let message = match js_sys::Reflect::get(thrown, &wasm_bindgen::JsValue::from_str("message")) {
        Ok(message) => message,
        Err(unreadable) => {
            tracing::warn!(error = ?unreadable, "thrown value carries no readable message");
            return BrowserError::Unreadable;
        }
    };
    let Some(text) = message.as_string() else {
        return BrowserError::Unreadable;
    };
    BrowserError::Js(text)
}
