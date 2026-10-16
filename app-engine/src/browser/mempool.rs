use crate::browser::source::Source;
use crate::config::{MEMPOOL_LISTED, MEMPOOL_SCANNED};
use crate::errors::BrowserError;
use crate::projection::{unconfirmed, Unconfirmed};
use crate::record::carries_urma;
use bitcoin::Txid;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use urma_wire::index::Entry;
use wire_app_core::model::Post;

enum Seen {
    Wire(Post),
    Other,
}

thread_local! { static SEEN: RefCell<BTreeMap<Txid, Seen>> = const { RefCell::new(BTreeMap::new()) }; }

pub async fn mempool(source: &Source) -> Result<Vec<Post>, BrowserError> {
    let listed = listed(source).await?;
    let fresh: Vec<Txid> = SEEN.with(|seen| {
        let mut seen = seen.borrow_mut();
        seen.retain(|txid, _seen| listed.contains(txid));
        listed
            .iter()
            .filter(|txid| !seen.contains_key(*txid))
            .take(MEMPOOL_SCANNED)
            .copied()
            .collect()
    });
    for txid in fresh {
        let held = scanned(source, txid).await;
        SEEN.with(|seen| seen.borrow_mut().insert(txid, held));
    }
    Ok(SEEN.with(|seen| {
        let mut posts = Vec::new();
        for held in seen.borrow().values() {
            let Seen::Wire(post) = held else {
                continue;
            };
            posts.push(post.clone());
        }
        posts
    }))
}

async fn listed(source: &Source) -> Result<BTreeSet<Txid>, BrowserError> {
    let rows: Vec<String> = serde_json::from_value(source.json("/mempool/txids").await?)?;
    if rows.len() > MEMPOOL_LISTED {
        return Err(BrowserError::Missing("a bounded mempool list".into()));
    }
    let mut listed = BTreeSet::new();
    for row in rows {
        listed.insert(row.parse::<Txid>()?);
    }
    Ok(listed)
}

async fn scanned(source: &Source, txid: Txid) -> Seen {
    match wire_post(source, txid).await {
        Ok(seen) => seen,
        Err(error) => {
            tracing::warn!(%txid, error = %error, "mempool transaction skipped");
            Seen::Other
        }
    }
}

async fn wire_post(source: &Source, txid: Txid) -> Result<Seen, BrowserError> {
    let reveal = source.transaction(txid).await?;
    if !carries_urma(&reveal) {
        return Ok(Seen::Other);
    }
    let commit = source
        .transaction(reveal.input[0].previous_output.txid)
        .await?;
    let verified = urma_profiles::wire::verify(&reveal, &commit)?;
    let entry = Entry {
        height: 0,
        position: 0,
        txid: txid.to_string(),
        author: verified.author().0.to_string(),
        record: hex::encode(verified.raw_record()),
        return_script: reveal
            .output
            .first()
            .map(|output| hex::encode(output.script_pubkey.as_bytes())),
    };
    Ok(match unconfirmed(&entry)? {
        Unconfirmed::Post(post) => Seen::Wire(post),
        Unconfirmed::Other => Seen::Other,
    })
}
