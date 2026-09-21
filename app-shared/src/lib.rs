//! Models, avatar rendering and adapters shared by the URMA Wire application.
//! Protocol, identity, wallet and file operations use the URMA libraries.

pub mod avatar;
pub mod codec;
pub mod connection;
pub mod identity;
pub use identity::IdentityStatus;
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod live;
#[cfg(not(target_arch = "wasm32"))]
pub mod live_index;
pub mod live_model;
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod persistent_store;
pub mod source;
pub mod store;
pub mod traits;

#[cfg(not(target_arch = "wasm32"))]
pub mod ledger;
#[cfg(not(target_arch = "wasm32"))]
pub mod wallet;

pub use avatar::{decode_avatar_pixels, render_avatar_svg, render_identicon_svg, AvatarPalette};
pub use codec::{ProtocolCodec, UrmaProtocolAdapter};
#[cfg(not(target_arch = "wasm32"))]
pub use ledger::LedgerClient;
pub use model::{
    AuthorId, AvatarRecord, Chain, ConfirmedStatus, Post, Profile, PublicationQuote, RecordKind,
    Reply, SpendableUtxo, Txid, WalletSummary, WireError,
};
#[cfg(not(target_arch = "wasm32"))]
pub use persistent_store::{BlockCheckpoint, PersistentStoreData, PersistentWireStore};
pub use source::MockChainSource;
pub use store::MockWireStore;
pub use traits::{ChainSource, ProtocolAdapter, WireStore};
#[cfg(not(target_arch = "wasm32"))]
pub use wallet::WireWallet;
