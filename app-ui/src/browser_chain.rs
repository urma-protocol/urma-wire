use super::{
    browser_source::{error, Source, CHAIN},
    browser_store as store,
};
use urma_core::{
    envelope,
    format::{PublicRecord, RecordKind},
};
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_shared::{live_model::Feed, AuthorId, Chain, ConfirmedStatus, Post, Txid};

const KEY: &str = "ltc-testnet-index-v2";
pub async fn cached() -> Result<Option<Index>, String> {
    let index: Option<Index> = store::read(KEY).await?;
    if let Some(index) = &index {
        if index.format != "URMA-WIRE-INDEX-2"
            || index.genesis != CHAIN.genesis().map_err(error)?.0.to_string()
            || index.blocks.len() > 500_000
            || index.entries.len() > 50_000
        {
            return Err("Invalid browser chain index".into());
        }
        for (offset, block) in index.blocks.iter().enumerate() {
            if block.height
                != index
                    .start
                    .checked_add(offset as u64)
                    .ok_or("Index height overflow")?
            {
                return Err("Noncontiguous browser checkpoints".into());
            }
            block.hash.parse::<bitcoin::BlockHash>().map_err(error)?;
        }
    }
    Ok(index)
}

pub async fn sync(source: &Source) -> Result<Feed, String> {
    let (tip, anchor) = source.tip().await?;
    let mut index = cached().await?.unwrap_or(Index {
        format: "URMA-WIRE-INDEX-2".into(),
        genesis: CHAIN.genesis().map_err(error)?.0.to_string(),
        start: tip.saturating_sub(100),
        blocks: vec![],
        entries: vec![],
    });
    while let Some(last) = index.blocks.last() {
        if last.height <= tip && source.hash(last.height).await?.to_string() == last.hash {
            break;
        }
        let height = last.height;
        index.entries.retain(|entry| entry.height < height);
        index.blocks.pop();
    }
    store::write(KEY, &index).await?;
    let next = index
        .start
        .checked_add(index.blocks.len() as u64)
        .ok_or("Index overflow")?;
    for height in next..=tip.min(next.saturating_add(99)) {
        let block = source.block(height).await?;
        if let Some(previous) = index.blocks.last() {
            if block.header.prev_blockhash.to_string() != previous.hash {
                return Err(
                    "Chain changed while indexing; refresh to roll back and continue".into(),
                );
            }
        }
        let mut entries = Vec::new();
        for (position, tx) in block.txdata.iter().enumerate() {
            if tx.input.len() != 1
                || !tx.input[0]
                    .witness
                    .iter()
                    .any(|item| item.windows(4).any(|bytes| bytes == b"URMA"))
            {
                continue;
            }
            let Ok(candidate) = envelope::extract_reveal(tx) else {
                continue;
            };
            if !matches!(
                RecordKind::parse(&candidate.record).map_err(error)?,
                RecordKind::Post
                    | RecordKind::Reply
                    | RecordKind::Profile
                    | RecordKind::Avatar
                    | RecordKind::WirePost
                    | RecordKind::WireReply
            ) {
                continue;
            }
            let commit = source.transaction(tx.input[0].previous_output.txid).await?;
            if let Ok(parsed) = urma_profiles::wire::verify(tx, &commit) {
                entries.push(Entry {
                    height,
                    position: position.try_into().map_err(error)?,
                    txid: tx.compute_txid().to_string(),
                    author: parsed.author().0.to_string(),
                    record: hex::encode(parsed.raw_record()),
                });
            }
        }
        if source.hash(height).await? != block.block_hash() {
            return Err("Chain changed during verification; refresh to continue".into());
        }
        index.entries.extend(entries);
        index.blocks.push(Checkpoint {
            height,
            hash: block.block_hash().to_string(),
        });
        if index.blocks.len() > 500_000 || index.entries.len() > 50_000 {
            return Err("Local chain index capacity reached".into());
        }
        // Each verified increment is durable before fetching the next block.
        store::write(KEY, &index).await?;
    }
    let stable = source.hash(tip).await? == anchor;
    project(
        &index,
        stable && index.blocks.last().map(|last| last.height) == Some(tip),
        None,
    )
}

pub fn project(index: &Index, complete: bool, reply_target: Option<&str>) -> Result<Feed, String> {
    let tip = index
        .blocks
        .last()
        .map(|block| block.height)
        .unwrap_or(index.start);
    let mut posts = Vec::new();
    let mut reply_targets = Vec::new();
    for entry in &index.entries {
        let (text, wire, hashtags, target) =
            match PublicRecord::decode(&hex::decode(&entry.record).map_err(error)?)
                .map_err(error)?
            {
                PublicRecord::Post(text) => (text, String::new(), vec![], None),
                PublicRecord::Reply { text, target } => {
                    (text, String::new(), vec![], Some(target.to_string()))
                }
                PublicRecord::WirePost { text, topics } => {
                    (text, topics.wire, topics.hashtags, None)
                }
                PublicRecord::WireReply {
                    text,
                    topics,
                    target,
                } => (text, topics.wire, topics.hashtags, Some(target.to_string())),
                _ => continue,
            };
        if let Some(target) = &target {
            reply_targets.push(target.clone());
        }
        if reply_target.is_some() && target.as_deref() != reply_target {
            continue;
        }
        let block = index
            .blocks
            .iter()
            .find(|block| block.height == entry.height)
            .ok_or("Record has no checkpoint")?;
        posts.push(Post {
            txid: Txid::from_display_hex(&entry.txid).map_err(error)?,
            author: AuthorId::from_hex(&entry.author).map_err(error)?,
            chain: Chain::Litecoin,
            text,
            wire,
            hashtags,
            status: ConfirmedStatus {
                block_height: entry.height,
                block_hash: block.hash.clone(),
                confirmations: tip
                    .checked_sub(entry.height)
                    .ok_or("Index height mismatch")?
                    + 1,
                tx_index: entry.position,
            },
            reply_count: 0,
        });
    }
    for post in &mut posts {
        post.reply_count = reply_targets
            .iter()
            .filter(|target| **target == post.txid.to_display_hex())
            .count();
    }
    posts.reverse();
    Ok(Feed {
        identities: wire_app_shared::public_identity::resolve(index.entries.iter().map(|entry| {
            (
                entry.height,
                entry.position,
                entry.author.as_str(),
                entry.record.as_str(),
            )
        }))?,
        posts,
        start: index.start,
        tip,
        complete_to_tip: complete,
    })
}
