use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    model::{AuthorId, AvatarRecord, Chain, Post, Profile, Reply, Txid, WireError},
    traits::WireStore,
};

/// Block metadata indexed by the local store to track chain continuity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockCheckpoint {
    pub height: u64,
    pub hash: String,
    pub parent_hash: String,
    pub txids: Vec<Txid>,
}

/// Serializable inner state of `PersistentWireStore`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PersistentStoreData {
    pub posts: Vec<Post>,
    pub replies: Vec<Reply>,
    pub profiles: Vec<Profile>,
    pub avatars: Vec<AvatarRecord>,
    pub chain_tips: HashMap<Chain, u64>,
    pub block_history: HashMap<Chain, Vec<BlockCheckpoint>>,
}

/// A crash-safe, persistent local WireStore backed by atomic disk writes.
///
/// Implements D-016 (atomic rollback upon reorg) and D-017 (confirmed records only >= 1 conf).
#[derive(Clone, Debug)]
pub struct PersistentWireStore {
    data: PersistentStoreData,
    storage_path: PathBuf,
}

impl PersistentWireStore {
    /// Opens an existing store file or initializes a new one.
    pub fn open_or_create(path: impl AsRef<Path>) -> Result<Self, WireError> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let bytes = urma_io::read_bounded(&path, 64 * 1024 * 1024)?;
            let data: PersistentStoreData =
                serde_json::from_slice(&bytes).map_err(|e| WireError::Io(e.to_string()))?;
            Ok(Self {
                data,
                storage_path: path,
            })
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let store = Self {
                data: PersistentStoreData::default(),
                storage_path: path,
            };
            store.save()?;
            Ok(store)
        }
    }

    /// Atomically persists store state to disk using a temporary file and atomic rename.
    pub fn save(&self) -> Result<(), WireError> {
        let bytes =
            serde_json::to_vec_pretty(&self.data).map_err(|e| WireError::Io(e.to_string()))?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(WireError::Io("app store exceeds 64 MiB".into()));
        }
        urma_io::write_replace(&self.storage_path, &bytes)?;
        Ok(())
    }

    /// Records an indexed block checkpoint and advances the tip.
    pub fn record_block(
        &mut self,
        chain: Chain,
        checkpoint: BlockCheckpoint,
    ) -> Result<(), WireError> {
        let height = checkpoint.height;
        let history = self.data.block_history.entry(chain).or_default();

        // Check continuity if we have prior blocks
        if let Some(prev) = history.last() {
            if checkpoint.parent_hash != prev.hash && checkpoint.height > prev.height {
                return Err(WireError::ReorgFailed(format!(
                    "chain continuity broken: parent {} does not match previous block {}",
                    checkpoint.parent_hash, prev.hash
                )));
            }
        }

        history.push(checkpoint);
        self.data.chain_tips.insert(chain, height);
        self.save()?;
        Ok(())
    }

    /// Returns the block checkpoints for the specified chain.
    pub fn get_block_history(&self, chain: Chain) -> &[BlockCheckpoint] {
        self.data
            .block_history
            .get(&chain)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

impl WireStore for PersistentWireStore {
    fn get_feed(&self, chain: Chain, limit: usize) -> Result<Vec<Post>, WireError> {
        let mut posts: Vec<Post> = self
            .data
            .posts
            .iter()
            .filter(|p| p.chain == chain && p.status.is_confirmed())
            .cloned()
            .collect();

        // Sort descending by (block_height, tx_index)
        posts.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });

        posts.truncate(limit);
        Ok(posts)
    }

    fn get_post(&self, chain: Chain, txid: &Txid) -> Result<Option<Post>, WireError> {
        Ok(self
            .data
            .posts
            .iter()
            .find(|p| p.chain == chain && &p.txid == txid && p.status.is_confirmed())
            .cloned())
    }

    fn get_replies(&self, chain: Chain, target_txid: &Txid) -> Result<Vec<Reply>, WireError> {
        let mut replies: Vec<Reply> = self
            .data
            .replies
            .iter()
            .filter(|r| {
                r.chain == chain && &r.target_txid == target_txid && r.status.is_confirmed()
            })
            .cloned()
            .collect();

        // Sort ascending by (block_height, tx_index) for replies
        replies.sort_by(|a, b| {
            a.status
                .block_height
                .cmp(&b.status.block_height)
                .then_with(|| a.status.tx_index.cmp(&b.status.tx_index))
        });

        Ok(replies)
    }

    fn get_profile(&self, chain: Chain, author: &AuthorId) -> Result<Option<Profile>, WireError> {
        // Latest valid confirmed profile wins (highest block_height then tx_index)
        let mut matching: Vec<&Profile> = self
            .data
            .profiles
            .iter()
            .filter(|p| p.chain == chain && &p.author == author && p.status.is_confirmed())
            .collect();

        matching.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });

        Ok(matching.first().copied().cloned())
    }

    fn get_avatar(
        &self,
        chain: Chain,
        author: &AuthorId,
    ) -> Result<Option<AvatarRecord>, WireError> {
        // Latest valid confirmed avatar wins
        let mut matching: Vec<&AvatarRecord> = self
            .data
            .avatars
            .iter()
            .filter(|a| a.chain == chain && &a.author == author && a.status.is_confirmed())
            .collect();

        matching.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });

        Ok(matching.first().copied().cloned())
    }

    fn get_author_posts(&self, chain: Chain, author: &AuthorId) -> Result<Vec<Post>, WireError> {
        let mut posts: Vec<Post> = self
            .data
            .posts
            .iter()
            .filter(|p| p.chain == chain && &p.author == author && p.status.is_confirmed())
            .cloned()
            .collect();

        posts.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });

        Ok(posts)
    }

    fn insert_post(&mut self, post: Post) -> Result<(), WireError> {
        // Enforce D-017: mempool (0 conf) cannot be ingested
        if !post.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }

        // Idempotently update or append post
        if let Some(existing) = self
            .data
            .posts
            .iter_mut()
            .find(|p| p.chain == post.chain && p.txid == post.txid)
        {
            *existing = post;
        } else {
            self.data.posts.push(post);
        }

        self.save()?;
        Ok(())
    }

    fn insert_reply(&mut self, reply: Reply) -> Result<(), WireError> {
        if !reply.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }

        let target_txid = reply.target_txid;
        let chain = reply.chain;

        if let Some(existing) = self
            .data
            .replies
            .iter_mut()
            .find(|r| r.chain == reply.chain && r.txid == reply.txid)
        {
            *existing = reply;
        } else {
            self.data.replies.push(reply);
        }

        // Increment reply count on parent post if present
        if let Some(parent) = self
            .data
            .posts
            .iter_mut()
            .find(|p| p.chain == chain && p.txid == target_txid)
        {
            parent.reply_count = self
                .data
                .replies
                .iter()
                .filter(|r| r.chain == chain && r.target_txid == target_txid)
                .count();
        }

        self.save()?;
        Ok(())
    }

    fn insert_profile(&mut self, profile: Profile) -> Result<(), WireError> {
        if !profile.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }

        self.data.profiles.push(profile);
        self.save()?;
        Ok(())
    }

    fn insert_avatar(&mut self, avatar: AvatarRecord) -> Result<(), WireError> {
        if !avatar.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }

        self.data.avatars.push(avatar);
        self.save()?;
        Ok(())
    }

    fn rollback_to_height(&mut self, chain: Chain, height: u64) -> Result<usize, WireError> {
        let initial_count = self
            .data
            .posts
            .iter()
            .filter(|p| p.chain == chain && p.status.block_height > height)
            .count()
            + self
                .data
                .replies
                .iter()
                .filter(|r| r.chain == chain && r.status.block_height > height)
                .count()
            + self
                .data
                .profiles
                .iter()
                .filter(|p| p.chain == chain && p.status.block_height > height)
                .count()
            + self
                .data
                .avatars
                .iter()
                .filter(|a| a.chain == chain && a.status.block_height > height)
                .count();

        // Remove orphaned records
        self.data
            .posts
            .retain(|p| !(p.chain == chain && p.status.block_height > height));
        self.data
            .replies
            .retain(|r| !(r.chain == chain && r.status.block_height > height));
        self.data
            .profiles
            .retain(|p| !(p.chain == chain && p.status.block_height > height));
        self.data
            .avatars
            .retain(|a| !(a.chain == chain && a.status.block_height > height));

        // Recompute reply counts for remaining posts
        for post in &mut self.data.posts {
            if post.chain == chain {
                post.reply_count = self
                    .data
                    .replies
                    .iter()
                    .filter(|r| r.chain == chain && r.target_txid == post.txid)
                    .count();
            }
        }

        // Roll back block history
        if let Some(history) = self.data.block_history.get_mut(&chain) {
            history.retain(|b| b.height <= height);
        }

        self.data.chain_tips.insert(chain, height);
        self.save()?;
        Ok(initial_count)
    }

    fn latest_indexed_height(&self, chain: Chain) -> Result<u64, WireError> {
        Ok(self.data.chain_tips.get(&chain).copied().unwrap_or(0))
    }
}
