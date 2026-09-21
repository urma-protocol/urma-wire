use crate::{
    model::{AuthorId, AvatarRecord, Chain, Post, Profile, Reply, Txid, WireError},
    traits::WireStore,
};
use std::collections::HashMap;

/// In-memory mock store implementing `WireStore`.
/// Supports isolated chain partitioning, reorg rollback, and confirmed-only queries.
#[derive(Clone, Debug, Default)]
pub struct MockWireStore {
    posts: Vec<Post>,
    replies: Vec<Reply>,
    profiles: Vec<Profile>,
    avatars: Vec<AvatarRecord>,
    chain_tips: HashMap<Chain, u64>,
}

impl MockWireStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WireStore for MockWireStore {
    fn get_feed(&self, chain: Chain, limit: usize) -> Result<Vec<Post>, WireError> {
        let mut list: Vec<Post> = self
            .posts
            .iter()
            .filter(|p| p.chain == chain && p.status.is_confirmed())
            .cloned()
            .collect();
        // Sort descending: highest block height first, then highest tx_index
        list.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });
        if list.len() > limit {
            list.truncate(limit);
        }
        Ok(list)
    }

    fn get_post(&self, chain: Chain, txid: &Txid) -> Result<Option<Post>, WireError> {
        Ok(self
            .posts
            .iter()
            .find(|p| p.chain == chain && p.txid == *txid && p.status.is_confirmed())
            .cloned())
    }

    fn get_replies(&self, chain: Chain, target_txid: &Txid) -> Result<Vec<Reply>, WireError> {
        let mut list: Vec<Reply> = self
            .replies
            .iter()
            .filter(|r| {
                r.chain == chain && r.target_txid == *target_txid && r.status.is_confirmed()
            })
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            a.status
                .block_height
                .cmp(&b.status.block_height)
                .then_with(|| a.status.tx_index.cmp(&b.status.tx_index))
        });
        Ok(list)
    }

    fn get_profile(&self, chain: Chain, author: &AuthorId) -> Result<Option<Profile>, WireError> {
        // Latest confirmed profile wins
        let mut profiles: Vec<Profile> = self
            .profiles
            .iter()
            .filter(|p| p.chain == chain && p.author == *author && p.status.is_confirmed())
            .cloned()
            .collect();
        profiles.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });
        Ok(profiles.into_iter().next())
    }

    fn get_avatar(
        &self,
        chain: Chain,
        author: &AuthorId,
    ) -> Result<Option<AvatarRecord>, WireError> {
        // Latest confirmed avatar wins
        let mut avatars: Vec<AvatarRecord> = self
            .avatars
            .iter()
            .filter(|a| a.chain == chain && a.author == *author && a.status.is_confirmed())
            .cloned()
            .collect();
        avatars.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });
        Ok(avatars.into_iter().next())
    }

    fn get_author_posts(&self, chain: Chain, author: &AuthorId) -> Result<Vec<Post>, WireError> {
        let mut list: Vec<Post> = self
            .posts
            .iter()
            .filter(|p| p.chain == chain && p.author == *author && p.status.is_confirmed())
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.status
                .block_height
                .cmp(&a.status.block_height)
                .then_with(|| b.status.tx_index.cmp(&a.status.tx_index))
        });
        Ok(list)
    }

    fn insert_post(&mut self, post: Post) -> Result<(), WireError> {
        if !post.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }
        self.posts.retain(|p| p.txid != post.txid);
        self.posts.push(post);
        Ok(())
    }

    fn insert_reply(&mut self, reply: Reply) -> Result<(), WireError> {
        if !reply.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }
        let target_txid = reply.target_txid;
        self.replies.retain(|r| r.txid != reply.txid);
        self.replies.push(reply);

        // Update post reply count if target is in store
        if let Some(post) = self.posts.iter_mut().find(|p| p.txid == target_txid) {
            post.reply_count += 1;
        }
        Ok(())
    }

    fn insert_profile(&mut self, profile: Profile) -> Result<(), WireError> {
        if !profile.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }
        self.profiles.push(profile);
        Ok(())
    }

    fn insert_avatar(&mut self, avatar: AvatarRecord) -> Result<(), WireError> {
        if !avatar.status.is_confirmed() {
            return Err(WireError::UnconfirmedMempool);
        }
        self.avatars.push(avatar);
        Ok(())
    }

    fn rollback_to_height(&mut self, chain: Chain, height: u64) -> Result<usize, WireError> {
        let prev_total =
            self.posts.len() + self.replies.len() + self.profiles.len() + self.avatars.len();

        self.posts
            .retain(|p| p.chain != chain || p.status.block_height <= height);
        self.replies
            .retain(|r| r.chain != chain || r.status.block_height <= height);
        self.profiles
            .retain(|p| p.chain != chain || p.status.block_height <= height);
        self.avatars
            .retain(|a| a.chain != chain || a.status.block_height <= height);

        let new_total =
            self.posts.len() + self.replies.len() + self.profiles.len() + self.avatars.len();
        Ok(prev_total - new_total)
    }

    fn latest_indexed_height(&self, chain: Chain) -> Result<u64, WireError> {
        Ok(self.chain_tips.get(&chain).copied().unwrap_or(0))
    }
}
