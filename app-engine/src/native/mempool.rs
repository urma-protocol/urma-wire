use crate::config::{MEMPOOL_LISTED, MEMPOOL_SCANNED};
use crate::errors::NativeError;
use crate::native::service::Service;
use crate::projection::{unconfirmed, Unconfirmed};
use crate::record::carries_urma;
use bitcoin::Txid;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};
use urma_core::multipart::VerifiedRecord;
use urma_runtime::node::Node;
use urma_wire::index::entry_from_verified;
use wire_app_core::model::Post;

pub(super) enum Seen {
    Wire(Post),
    Other,
}

pub(super) struct Pool(Mutex<BTreeMap<Txid, Seen>>);

impl Pool {
    pub(super) fn empty() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<Txid, Seen>> {
        match self.0.lock() {
            Ok(held) => held,
            Err(poisoned) => {
                tracing::error!("mempool cache lock poisoned; reusing the held cache");
                poisoned.into_inner()
            }
        }
    }
}

impl Service {
    pub fn mempool(&self) -> Result<Vec<Post>, NativeError> {
        self.with_node(|node| {
            let listed = listed(node)?;
            let mut seen = self.mempool.held();
            seen.retain(|txid, _seen| listed.contains(txid));
            let fresh: Vec<Txid> = listed
                .iter()
                .filter(|txid| !seen.contains_key(*txid))
                .take(MEMPOOL_SCANNED)
                .copied()
                .collect();
            for txid in fresh {
                seen.insert(txid, scanned(node, txid));
            }
            let mut posts = Vec::new();
            for held in seen.values() {
                let Seen::Wire(post) = held else {
                    continue;
                };
                posts.push(post.clone());
            }
            Ok(posts)
        })
    }
}

fn listed(node: &Node) -> Result<BTreeSet<Txid>, NativeError> {
    let observed = node.observe("getrawmempool", &[])?;
    let Some(rows) = observed.value.as_array() else {
        return Err(NativeError::Mempool(
            "the source answered no transaction list",
        ));
    };
    if rows.len() > MEMPOOL_LISTED {
        return Err(NativeError::Mempool(
            "the source listed more transactions than allowed",
        ));
    }
    let mut listed = BTreeSet::new();
    for row in rows {
        let Some(txid) = row.as_str() else {
            return Err(NativeError::Mempool(
                "the source listed a non-text transaction id",
            ));
        };
        listed.insert(txid.parse::<Txid>()?);
    }
    Ok(listed)
}

fn scanned(node: &Node, txid: Txid) -> Seen {
    match wire_post(node, txid) {
        Ok(seen) => seen,
        Err(error) => {
            tracing::warn!(%txid, error = %error, "mempool transaction skipped");
            Seen::Other
        }
    }
}

fn wire_post(node: &Node, txid: Txid) -> Result<Seen, NativeError> {
    let reveal = node.transaction(txid)?;
    if !carries_urma(&reveal) {
        return Ok(Seen::Other);
    }
    let commit = node.transaction(reveal.input[0].previous_output.txid)?;
    let proof = VerifiedRecord::verify(txid, &reveal, &commit)?;
    let script = reveal
        .output
        .first()
        .map(|output| output.script_pubkey.as_script());
    let Some(entry) = entry_from_verified(&proof, script, 0, 0)? else {
        return Ok(Seen::Other);
    };
    Ok(match unconfirmed(&entry)? {
        Unconfirmed::Post(post) => Seen::Wire(post),
        Unconfirmed::Other => Seen::Other,
    })
}
