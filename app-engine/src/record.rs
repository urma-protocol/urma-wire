use crate::errors::RecordError;
use bitcoin::hashes::Hash;
use urma_core::{format::PublicRecord, topics::Topics};
use wire_app_core::avatar::Urma;
use wire_app_core::model::Txid;
use wire_app_core::publication::{Addressed, Content};

pub fn build(content: &Content) -> Result<PublicRecord, RecordError> {
    let record = match content {
        Content::Nickname(nickname) => PublicRecord::Profile(nickname.clone()),
        Content::Avatar(pixels) => match <[u8; Urma::AVATAR_BYTES]>::try_from(pixels.as_slice()) {
            Ok(pixels) => PublicRecord::Avatar(Box::new(pixels)),
            Err(rejected) => {
                tracing::warn!(error = %rejected, bytes = pixels.len(), "avatar refused");
                return Err(RecordError::AvatarSize);
            }
        },
        Content::Post {
            text,
            wire,
            hashtags,
            addressed,
        } => spoken(text, wire, hashtags, *addressed)?,
    };
    record.encode()?;
    Ok(record)
}

fn spoken(
    text: &str,
    wire: &str,
    hashtags: &[String],
    addressed: Addressed,
) -> Result<PublicRecord, RecordError> {
    let topics = Topics {
        wire: wire.to_owned(),
        hashtags: hashtags.to_vec(),
    };
    topics.validate()?;
    if text.trim().is_empty() {
        return Err(RecordError::EmptyText);
    }
    let text = text.to_owned();
    let structured = !topics.wire.is_empty() || !topics.hashtags.is_empty();
    Ok(match (addressed, structured) {
        (Addressed::Thread(target), true) => PublicRecord::WireReply {
            target: bitcoin_txid(target),
            topics,
            text,
        },
        (Addressed::Standalone, true) => PublicRecord::WirePost { topics, text },
        (Addressed::Thread(target), false) => PublicRecord::Reply {
            target: bitcoin_txid(target),
            text,
        },
        (Addressed::Standalone, false) => PublicRecord::Post(text),
    })
}

pub fn review(bytes: &[u8]) -> Result<Content, RecordError> {
    Ok(match PublicRecord::decode(bytes)? {
        PublicRecord::Post(text) => post(text, untagged(), Addressed::Standalone),
        PublicRecord::Reply { text, target } => {
            post(text, untagged(), Addressed::Thread(wire_txid(target)))
        }
        PublicRecord::WirePost { text, topics } => post(text, topics, Addressed::Standalone),
        PublicRecord::WireReply {
            text,
            topics,
            target,
        } => post(text, topics, Addressed::Thread(wire_txid(target))),
        PublicRecord::Profile(nickname) => Content::Nickname(nickname),
        PublicRecord::Avatar(pixels) => Content::Avatar(pixels.to_vec()),
        foreign @ PublicRecord::ProfileRecord { .. } => {
            return Err(RecordError::NotWire(foreign.kind().byte()))
        }
    })
}

fn untagged() -> Topics {
    Topics {
        wire: String::new(),
        hashtags: Vec::new(),
    }
}

fn post(text: String, topics: Topics, addressed: Addressed) -> Content {
    Content::Post {
        text,
        wire: topics.wire,
        hashtags: topics.hashtags,
        addressed,
    }
}

pub fn carries_urma(tx: &bitcoin::Transaction) -> bool {
    tx.input.len() == 1
        && tx.input[0]
            .witness
            .iter()
            .any(|item| item.windows(4).any(|bytes| bytes == b"URMA"))
}

pub fn bitcoin_txid(txid: Txid) -> bitcoin::Txid {
    bitcoin::Txid::from_byte_array(txid.0)
}

pub fn wire_txid(txid: bitcoin::Txid) -> Txid {
    Txid::from_raw_bytes(txid.to_byte_array())
}
