use crate::errors::ProjectionError;
use crate::identities::{gathered, Named, Naming};
use crate::record::wire_txid;
use urma_core::format::PublicRecord;
use urma_wire::cached::{decode_cached, CachedRecord};
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_core::feed::{Feed, FeedSource, Lookup};
use wire_app_core::model::{AuthorId, Chain, ConfirmedStatus, Inclusion, Placement, Post, Txid};

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

pub struct View<'a> {
    pub start: u64,
    pub tip: u64,
    pub blocks: &'a [Checkpoint],
    pub entries: &'a [Entry],
}

pub fn project(index: &Index, complete_to_tip: bool) -> Result<Feed, ProjectionError> {
    project_view(
        &View {
            start: index.start,
            tip: tip_of(index),
            blocks: &index.blocks,
            entries: &index.entries,
        },
        complete_to_tip,
    )
}

pub fn project_view(view: &View<'_>, complete_to_tip: bool) -> Result<Feed, ProjectionError> {
    let tip = view.tip;
    let mut posts = Vec::new();
    let mut namings = Vec::new();
    for entry in view.entries {
        let record = match decode_cached(&hex::decode(&entry.record)?)? {
            CachedRecord::Current(record) => record,
            CachedRecord::LegacyProfile => {
                tracing::warn!(txid = %entry.txid, "unsupported legacy profile omitted from feed projection; cached bytes retained");
                continue;
            }
            CachedRecord::LegacyAvatar => {
                tracing::warn!(txid = %entry.txid, "unsupported legacy avatar omitted from feed projection; cached bytes retained");
                continue;
            }
        };
        match decoded(record) {
            Decoded::Said(said) => posts.push(confirmed(view, entry, said)?),
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
        start: view.start,
        tip,
        complete_to_tip,
        source: FeedSource::AutonomousLocal,
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

fn confirmed(view: &View<'_>, entry: &Entry, said: Said) -> Result<Post, ProjectionError> {
    let Some(block) = view
        .blocks
        .iter()
        .find(|block| block.height == entry.height)
    else {
        return Err(ProjectionError::NoCheckpoint);
    };
    let Some(behind) = view.tip.checked_sub(entry.height) else {
        return Err(ProjectionError::HeightAboveTip);
    };
    let inclusion = Inclusion::Confirmed(ConfirmedStatus {
        block_height: entry.height,
        block_hash: block.hash.clone(),
        confirmations: behind + 1,
        tx_index: entry.position,
    });
    written(entry, said, inclusion)
}

fn written(entry: &Entry, said: Said, inclusion: Inclusion) -> Result<Post, ProjectionError> {
    Ok(Post {
        txid: Txid::from_display_hex(&entry.txid)?,
        author: AuthorId::from_hex(&entry.author)?,
        chain: Chain::Litecoin,
        text: said.text,
        wire: said.wire,
        hashtags: said.hashtags,
        placement: said.placement,
        inclusion,
        reply_count: 0,
    })
}

pub enum Unconfirmed {
    Post(Post),
    Other,
}

pub fn unconfirmed(entry: &Entry) -> Result<Unconfirmed, ProjectionError> {
    let record = match decode_cached(&hex::decode(&entry.record)?)? {
        CachedRecord::Current(record) => record,
        CachedRecord::LegacyProfile | CachedRecord::LegacyAvatar => return Ok(Unconfirmed::Other),
    };
    match decoded(record) {
        Decoded::Said(said) => Ok(Unconfirmed::Post(written(entry, said, Inclusion::Mempool)?)),
        Decoded::Named(_) | Decoded::Foreign(_) => Ok(Unconfirmed::Other),
    }
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
