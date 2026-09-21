use crate::{
    live_model::Feed,
    model::{AuthorId, Chain, ConfirmedStatus, Post, Txid},
};
use bitcoin::{Block, BlockHash, Transaction};
use urma_core::format::PublicRecord;
use urma_runtime::{error::Error, node::Node};
use urma_wire::{index::Index, reader::Reader};

pub struct NodeReader(pub Node);
impl Reader for NodeReader {
    type Error = Error;
    fn genesis(&self) -> Result<BlockHash, Error> {
        Ok(self.0.chain().genesis()?.0)
    }
    fn tip_height(&self) -> Result<u64, Error> {
        self.0.tip_height()
    }
    fn block_hash(&self, height: u64) -> Result<BlockHash, Error> {
        self.0.block_hash(height)
    }
    fn block(&self, height: u64) -> Result<Block, Error> {
        self.0.block(height)
    }
    fn transaction(&self, txid: bitcoin::Txid) -> Result<Transaction, Error> {
        self.0.transaction(txid)
    }
}

pub fn project(index: &Index, complete_to_tip: bool) -> Result<Feed, String> {
    let tip = index
        .blocks
        .last()
        .map(|block| block.height)
        .unwrap_or(index.start);
    let mut posts = Vec::new();
    let mut replies = Vec::new();
    for entry in &index.entries {
        let record = PublicRecord::decode(&hex::decode(&entry.record).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let (text, wire, hashtags) = match record {
            PublicRecord::Post(text) => (text, String::new(), Vec::new()),
            PublicRecord::Reply { target, text } => {
                replies.push(target.to_string());
                (text, String::new(), Vec::new())
            }
            PublicRecord::WirePost { topics, text } => (text, topics.wire, topics.hashtags),
            PublicRecord::WireReply {
                target,
                topics,
                text,
            } => {
                replies.push(target.to_string());
                (text, topics.wire, topics.hashtags)
            }
            _ => continue,
        };
        let block = index
            .blocks
            .iter()
            .find(|block| block.height == entry.height)
            .ok_or("Indexed record has no block checkpoint")?;
        posts.push(Post {
            txid: Txid::from_display_hex(&entry.txid).map_err(|e| e.to_string())?,
            author: AuthorId::from_hex(&entry.author).map_err(|e| e.to_string())?,
            chain: Chain::Litecoin,
            text,
            wire,
            hashtags,
            status: ConfirmedStatus {
                block_height: entry.height,
                block_hash: block.hash.clone(),
                confirmations: tip
                    .checked_sub(entry.height)
                    .ok_or("Invalid index height")?
                    + 1,
                tx_index: entry.position,
            },
            reply_count: 0,
        });
    }
    for post in &mut posts {
        post.reply_count = replies
            .iter()
            .filter(|target| **target == post.txid.to_display_hex())
            .count();
    }
    posts.reverse();
    Ok(Feed {
        posts,
        start: index.start,
        tip,
        complete_to_tip,
    })
}

pub fn replies(index: &Index, target: &str) -> Result<Vec<Post>, String> {
    let targets: Result<Vec<_>, String> = index
        .entries
        .iter()
        .map(|entry| {
            let bytes = hex::decode(&entry.record).map_err(|e| e.to_string())?;
            Ok((
                entry.txid.clone(),
                PublicRecord::decode(&bytes).map_err(|e| e.to_string())?,
            ))
        })
        .collect();
    let ids: Vec<_> = targets?
        .into_iter()
        .filter_map(|(id, record)| match record {
            PublicRecord::Reply {
                target: reply_target,
                ..
            }
            | PublicRecord::WireReply {
                target: reply_target,
                ..
            } if reply_target.to_string() == target => Some(id),
            _ => None,
        })
        .collect();
    Ok(project(index, false)?
        .posts
        .into_iter()
        .filter(|post| ids.contains(&post.txid.to_display_hex()))
        .collect())
}
