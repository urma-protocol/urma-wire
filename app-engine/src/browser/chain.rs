use crate::browser::source::Source;
use crate::browser::store::{self, Stored};
use crate::config::{
    bounded_slice, HISTORY_BLOCKS, INDEX_FORMAT, INDEX_KEY, MAX_CHECKPOINTS, MAX_ENTRIES, NETWORK,
};
use crate::errors::BrowserError;
use bitcoin::{Block, Transaction};
use urma_core::envelope;
use urma_core::format::RecordKind;
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_core::feed::SyncProgress;

pub enum Cached {
    Index(Index),
    Empty,
}

pub struct Refreshed {
    pub index: Index,
    pub progress: SyncProgress,
}

pub async fn cached() -> Result<Cached, BrowserError> {
    let Stored::Found(index) = store::read::<Index>(INDEX_KEY).await? else {
        return Ok(Cached::Empty);
    };
    validated(&index)?;
    Ok(Cached::Index(index))
}

fn validated(index: &Index) -> Result<(), BrowserError> {
    if index.format != INDEX_FORMAT
        || index.genesis != NETWORK.genesis()?.0.to_string()
        || index.blocks.len() > MAX_CHECKPOINTS
        || index.entries.len() > MAX_ENTRIES
    {
        return Err(BrowserError::InvalidIndex);
    }
    for (offset, block) in index.blocks.iter().enumerate() {
        let expected = index
            .start
            .checked_add(u64::try_from(offset)?)
            .ok_or(BrowserError::HeightOverflow)?;
        if block.height != expected {
            return Err(BrowserError::Noncontiguous);
        }
        block.hash.parse::<bitcoin::BlockHash>()?;
    }
    Ok(())
}

fn fresh(tip: u64) -> Result<Index, BrowserError> {
    Ok(Index {
        format: INDEX_FORMAT.into(),
        genesis: NETWORK.genesis()?.0.to_string(),
        start: tip.max(HISTORY_BLOCKS) - HISTORY_BLOCKS,
        blocks: vec![],
        entries: vec![],
    })
}

pub async fn sync(source: &Source, max_blocks: u64) -> Result<Refreshed, BrowserError> {
    let (tip, anchor) = source.tip().await?;
    let mut index = match cached().await? {
        Cached::Index(index) => index,
        Cached::Empty => fresh(tip)?,
    };
    let rolled_back = roll_back(source, &mut index, tip).await?;
    store::write(INDEX_KEY, &index).await?;
    let next = index
        .start
        .checked_add(u64::try_from(index.blocks.len())?)
        .ok_or(BrowserError::HeightOverflow)?;
    let horizon = next
        .checked_add(bounded_slice(max_blocks) - 1)
        .ok_or(BrowserError::HeightOverflow)?;
    let mut scanned = 0;
    for height in next..=tip.min(horizon) {
        extend(source, &mut index, height).await?;
        scanned += 1;
    }
    let stable = source.hash(tip).await? == anchor;
    let progress = SyncProgress {
        start: index.start,
        indexed_to: index.blocks.last().map(|block| block.height),
        tip,
        scanned,
        records: u64::try_from(index.entries.len())?,
        rolled_back,
        complete_to_tip: stable && reached(&index, tip),
    };
    Ok(Refreshed { index, progress })
}

async fn roll_back(source: &Source, index: &mut Index, tip: u64) -> Result<u64, BrowserError> {
    let mut removed = 0;
    loop {
        let Some(last) = index.blocks.last() else {
            return Ok(removed);
        };
        if last.height <= tip && source.hash(last.height).await?.to_string() == last.hash {
            return Ok(removed);
        }
        let Some(dropped) = index.blocks.pop() else {
            return Ok(removed);
        };
        index.entries.retain(|entry| entry.height < dropped.height);
        removed += 1;
    }
}

async fn extend(source: &Source, index: &mut Index, height: u64) -> Result<(), BrowserError> {
    let block = source.block(height).await?;
    linked(index, &block)?;
    let entries = verified_entries(source, &block, height).await?;
    if source.hash(height).await? != block.block_hash() {
        return Err(BrowserError::ChangedDuringVerification);
    }
    index.entries.extend(entries);
    index.blocks.push(Checkpoint {
        height,
        hash: block.block_hash().to_string(),
    });
    if index.blocks.len() > MAX_CHECKPOINTS || index.entries.len() > MAX_ENTRIES {
        return Err(BrowserError::IndexCapacity);
    }
    store::write(INDEX_KEY, index).await
}

fn linked(index: &Index, block: &Block) -> Result<(), BrowserError> {
    let Some(previous) = index.blocks.last() else {
        return Ok(());
    };
    if block.header.prev_blockhash.to_string() != previous.hash {
        return Err(BrowserError::ReorgWhileIndexing);
    }
    Ok(())
}

fn reached(index: &Index, tip: u64) -> bool {
    let Some(last) = index.blocks.last() else {
        return false;
    };
    last.height == tip
}

fn carries_urma(tx: &Transaction) -> bool {
    tx.input.len() == 1
        && tx.input[0]
            .witness
            .iter()
            .any(|item| item.windows(4).any(|bytes| bytes == b"URMA"))
}

fn wire_kind(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::Post
            | RecordKind::Reply
            | RecordKind::Profile
            | RecordKind::Avatar
            | RecordKind::WirePost
            | RecordKind::WireReply
    )
}

async fn verified_entries(
    source: &Source,
    block: &Block,
    height: u64,
) -> Result<Vec<Entry>, BrowserError> {
    let mut entries = Vec::new();
    for (position, tx) in block.txdata.iter().enumerate() {
        if !carries_urma(tx) {
            continue;
        }
        let candidate = match envelope::extract_reveal(tx) {
            Ok(candidate) => candidate,
            Err(error) => {
                tracing::warn!(error = %error, "candidate is not a supported URMA reveal");
                continue;
            }
        };
        if !wire_kind(RecordKind::parse(&candidate.record)?) {
            continue;
        }
        let commit = source.transaction(tx.input[0].previous_output.txid).await?;
        match urma_profiles::wire::verify(tx, &commit) {
            Ok(verified) => entries.push(Entry {
                height,
                position: u32::try_from(position)?,
                txid: tx.compute_txid().to_string(),
                author: verified.author().0.to_string(),
                record: hex::encode(verified.raw_record()),
            }),
            Err(error) => tracing::warn!(error = %error, "rejected Wire author proof"),
        }
    }
    Ok(entries)
}
