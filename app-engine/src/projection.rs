use crate::errors::ProjectionError;
use crate::identities::{gathered, Named, Naming};
use crate::record::wire_txid;
use urma_core::format::PublicRecord;
use urma_wire::cached::{decode_cached, CachedRecord};
use urma_wire::index::{Entry, Index};
use wire_app_core::feed::{Feed, Lookup};
use wire_app_core::model::{AuthorId, Chain, ConfirmedStatus, Placement, Post, Txid};

struct Said {
    text: String,
    wire: String,
    hashtags: Vec<String>,
    placement: Placement,
}

enum Decoded {
    Said(Said),
    Named(Named),
    Foreign(u8),
}

fn decoded(record: PublicRecord) -> Decoded {
    match record {
        PublicRecord::Post(text) => Decoded::Said(Said {
            text,
            wire: String::new(),
            hashtags: Vec::new(),
            placement: Placement::Root,
        }),
        PublicRecord::Reply { target, text } => Decoded::Said(Said {
            text,
            wire: String::new(),
            hashtags: Vec::new(),
            placement: Placement::Reply(wire_txid(target)),
        }),
        PublicRecord::WirePost { topics, text } => Decoded::Said(Said {
            text,
            wire: topics.wire,
            hashtags: topics.hashtags,
            placement: Placement::Root,
        }),
        PublicRecord::WireReply {
            target,
            topics,
            text,
        } => Decoded::Said(Said {
            text,
            wire: topics.wire,
            hashtags: topics.hashtags,
            placement: Placement::Reply(wire_txid(target)),
        }),
        PublicRecord::Profile(nickname) => Decoded::Named(Named::Nickname(nickname)),
        PublicRecord::Avatar(pixels) => Decoded::Named(Named::Avatar(pixels.to_vec())),
        foreign @ PublicRecord::ProfileRecord { .. } => Decoded::Foreign(foreign.kind().byte()),
    }
}

pub fn project(index: &Index, complete_to_tip: bool) -> Result<Feed, ProjectionError> {
    let tip = tip_of(index);
    let mut posts = Vec::new();
    let mut namings = Vec::new();
    for entry in &index.entries {
        let record = match decode_cached(&hex::decode(&entry.record)?)? {
            CachedRecord::Current(record) => record,
            CachedRecord::LegacyAvatar => {
                tracing::warn!(txid = %entry.txid, "unsupported legacy avatar omitted from feed projection; cached bytes retained");
                continue;
            }
        };
        match decoded(record) {
            Decoded::Said(said) => posts.push(confirmed(index, entry, tip, said)?),
            Decoded::Named(named) => namings.push(Naming {
                height: entry.height,
                position: entry.position,
                author: entry.author.clone(),
                named,
            }),
            Decoded::Foreign(kind) => {
                tracing::warn!(kind, txid = %entry.txid, "indexed record is not a Wire record")
            }
        }
    }
    count_replies(&mut posts);
    posts.reverse();
    Ok(Feed {
        identities: gathered(namings),
        posts,
        start: index.start,
        tip,
        complete_to_tip,
    })
}

pub fn lookup(index: &Index, txid: Txid) -> Result<Lookup, ProjectionError> {
    let feed = project(index, false)?;
    let Some(post) = feed.posts.into_iter().find(|post| post.txid == txid) else {
        return Ok(Lookup::Absent);
    };
    Ok(Lookup::Found(post))
}

pub fn replies(index: &Index, target: Txid) -> Result<Vec<Post>, ProjectionError> {
    Ok(project(index, false)?
        .posts
        .into_iter()
        .filter(|post| post.placement == Placement::Reply(target))
        .collect())
}

fn tip_of(index: &Index) -> u64 {
    let Some(last) = index.blocks.last() else {
        return index.start;
    };
    last.height
}

fn confirmed(index: &Index, entry: &Entry, tip: u64, said: Said) -> Result<Post, ProjectionError> {
    let Some(block) = index
        .blocks
        .iter()
        .find(|block| block.height == entry.height)
    else {
        return Err(ProjectionError::NoCheckpoint);
    };
    let Some(behind) = tip.checked_sub(entry.height) else {
        return Err(ProjectionError::HeightAboveTip);
    };
    Ok(Post {
        txid: Txid::from_display_hex(&entry.txid)?,
        author: AuthorId::from_hex(&entry.author)?,
        chain: Chain::Litecoin,
        text: said.text,
        wire: said.wire,
        hashtags: said.hashtags,
        placement: said.placement,
        status: ConfirmedStatus {
            block_height: entry.height,
            block_hash: block.hash.clone(),
            confirmations: behind + 1,
            tx_index: entry.position,
        },
        reply_count: 0,
    })
}

fn count_replies(posts: &mut [Post]) {
    let mut targets: Vec<Txid> = Vec::new();
    for post in posts.iter() {
        let Placement::Reply(target) = post.placement else {
            continue;
        };
        targets.push(target);
    }
    targets.sort();
    for post in posts.iter_mut() {
        let first = targets.partition_point(|target| *target < post.txid);
        let past = targets.partition_point(|target| *target <= post.txid);
        post.reply_count = past - first;
    }
}
