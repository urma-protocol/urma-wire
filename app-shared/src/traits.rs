use crate::model::{
    AuthorId, AvatarRecord, Chain, ConfirmedStatus, Post, Profile, Reply, Txid, WireError,
};

/// Boundary for local indexed storage and identity cache.
///
/// Ingests only confirmed records (>= 1 confirmation, D-017).
/// Supports rollback upon blockchain reorg (D-016).
/// Operates strictly within a single chain context.
pub trait WireStore: Send + Sync {
    /// Retrieve latest confirmed posts for the feed, sorted descending by block height and tx index.
    fn get_feed(&self, chain: Chain, limit: usize) -> Result<Vec<Post>, WireError>;

    /// Retrieve a single post by txid within the specified chain.
    fn get_post(&self, chain: Chain, txid: &Txid) -> Result<Option<Post>, WireError>;

    /// Retrieve all replies for a target post txid within the specified chain.
    fn get_replies(&self, chain: Chain, target_txid: &Txid) -> Result<Vec<Reply>, WireError>;

    /// Retrieve the latest valid confirmed profile for an author in the specified chain.
    fn get_profile(&self, chain: Chain, author: &AuthorId) -> Result<Option<Profile>, WireError>;

    /// Retrieve the latest valid confirmed avatar for an author in the specified chain.
    fn get_avatar(
        &self,
        chain: Chain,
        author: &AuthorId,
    ) -> Result<Option<AvatarRecord>, WireError>;

    /// Retrieve all posts authored by a specific author in the specified chain.
    fn get_author_posts(&self, chain: Chain, author: &AuthorId) -> Result<Vec<Post>, WireError>;

    /// Insert a confirmed post into the store.
    fn insert_post(&mut self, post: Post) -> Result<(), WireError>;

    /// Insert a confirmed reply into the store.
    fn insert_reply(&mut self, reply: Reply) -> Result<(), WireError>;

    /// Update or insert an author's profile (latest block/tx position wins).
    fn insert_profile(&mut self, profile: Profile) -> Result<(), WireError>;

    /// Update or insert an author's avatar (latest block/tx position wins).
    fn insert_avatar(&mut self, avatar: AvatarRecord) -> Result<(), WireError>;

    /// Roll back all records beyond `height` (common ancestor during a chain reorg).
    /// Returns the number of invalidated records.
    fn rollback_to_height(&mut self, chain: Chain, height: u64) -> Result<usize, WireError>;

    /// Get the highest indexed block height for the given chain.
    fn latest_indexed_height(&self, chain: Chain) -> Result<u64, WireError>;
}

/// Boundary for querying blockchain state (Esplora, Electrum, or local node RPC).
pub trait ChainSource: Send + Sync {
    fn chain(&self) -> Chain;
    fn get_tip_height(&self) -> Result<u64, WireError>;
    fn get_block_hash(&self, height: u64) -> Result<String, WireError>;
    fn check_confirmation(&self, txid: &Txid) -> Result<Option<ConfirmedStatus>, WireError>;
}

/// Boundary for encoding and decoding raw URMA V0 public records (§7).
pub trait ProtocolAdapter: Send + Sync {
    fn encode_post(&self, text: &str) -> Result<Vec<u8>, WireError>;
    fn decode_post(&self, bytes: &[u8]) -> Result<String, WireError>;

    fn encode_reply(&self, target: &Txid, text: &str) -> Result<Vec<u8>, WireError>;
    fn decode_reply(&self, bytes: &[u8]) -> Result<(Txid, String), WireError>;

    fn encode_profile(&self, pseudonym: &str) -> Result<Vec<u8>, WireError>;
    fn decode_profile(&self, bytes: &[u8]) -> Result<String, WireError>;

    fn encode_avatar(&self, pixels: &[u8; 512]) -> Result<Vec<u8>, WireError>;
    fn decode_avatar(&self, bytes: &[u8]) -> Result<[u8; 512], WireError>;
}
