//! Optional local-operator feed acceleration. No daemon startup or chain reader.
//! Pages are a trusted local index, not portable verification evidence. A failed
//! traversal is discarded; callers can continue with the autonomous scanner.
use crate::config::{ArchiveSettings, NETWORK};
use crate::errors::NativeError;
use crate::projection::{project_view, View};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use urma_node_client::model::{ArchiveStatus, SCAN_RULES_VERSION};
use urma_node_client::Client;
use urma_wire::index::Checkpoint;
use wire_app_core::feed::Feed;

const MAX_RECORDS: usize = 50_000;
const MAX_RECORD_HEX: usize = 64 * 1024 * 1024;
const TRAVERSAL_TIME: Duration = Duration::from_secs(30);

pub(super) struct Snapshot {
    pub feed: Feed,
    pub archive: ArchiveStatus,
}

fn refused(message: &str) -> NativeError {
    NativeError::Archive(message.into())
}

/// Bound the traversal separately from each client's five-second operation.
/// An in-flight operation is not preempted. No partially read snapshot escapes.
pub(super) fn read(settings: &ArchiveSettings) -> Result<Snapshot, NativeError> {
    let deadline = Instant::now() + TRAVERSAL_TIME;
    if !std::fs::symlink_metadata(&settings.token_file)?.is_file() {
        return Err(refused("token must be a regular private file"));
    }
    let token = urma_io::read_private(&settings.token_file, 256)?;
    let token = std::str::from_utf8(&token)
        .map_err(|_| refused("token must be UTF-8"))?
        .trim();
    let client = Client::new(settings.endpoint, token.to_owned())
        .map_err(|_| refused("invalid loopback endpoint or bearer token"))?;
    let mut cursor = None;
    let mut expected_archive = None;
    let mut archive_bytes = None;
    let mut entries = Vec::new();
    let mut placements = BTreeMap::new();
    let mut hex_bytes = 0usize;
    loop {
        if Instant::now() >= deadline {
            return Err(refused("feed traversal deadline"));
        }
        // Do not copy server-controlled JSON diagnostics into logs: they could
        // reflect a bearer. Failure falls back without exposing response data.
        let page = client
            .wire_page(cursor.as_ref(), 128)
            .map_err(|_| refused("feed unavailable or snapshot changed"))?;
        if Instant::now() >= deadline {
            return Err(refused("feed traversal deadline"));
        }
        if page.archive.chain_genesis != NETWORK.genesis()?.0
            || page.archive.scan_rules_version != SCAN_RULES_VERSION
        {
            return Err(refused("wrong chain or scan rules"));
        }
        let encoded = serde_json::to_vec(&page.archive)?;
        if archive_bytes
            .as_ref()
            .is_some_and(|previous| *previous != encoded)
        {
            return Err(refused("archive status changed within snapshot"));
        }
        if let Some(indexed) = page.archive.indexed_through {
            let Some(tip) = page.archive.selected_tip else {
                return Err(refused("coverage without selected headers"));
            };
            if indexed.height < page.archive.application_start.height
                || indexed.height > tip.height
                || (indexed.height == tip.height && indexed.hash != tip.hash)
                || (indexed.height == page.archive.application_start.height
                    && indexed.hash != page.archive.application_start.hash)
            {
                return Err(refused("inconsistent archive coverage"));
            }
        }
        if entries.len() + page.records.len() > MAX_RECORDS {
            return Err(refused("feed record capacity exceeded"));
        }
        for record in page.records {
            hex_bytes = hex_bytes
                .checked_add(record.entry.record.len())
                .ok_or_else(|| refused("feed byte capacity exceeded"))?;
            if hex_bytes > MAX_RECORD_HEX {
                return Err(refused("feed byte capacity exceeded"));
            }
            if placements
                .insert(record.entry.height, record.block_hash)
                .is_some_and(|previous| previous != record.block_hash)
            {
                return Err(refused("conflicting placement across pages"));
            }
            entries.push(record.entry);
        }
        if expected_archive.is_none() {
            archive_bytes = Some(encoded);
            expected_archive = Some(page.archive);
        }
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    let archive = expected_archive.ok_or_else(|| refused("missing archive snapshot"))?;
    let complete =
        archive.selected_tip.is_some() && archive.indexed_through == archive.selected_tip;
    let blocks: Vec<_> = placements
        .into_iter()
        .map(|(height, hash)| Checkpoint {
            height,
            hash: hash.to_string(),
        })
        .collect();
    let feed = project_view(
        &View {
            start: archive.application_start.height,
            tip: archive
                .selected_tip
                .map_or(archive.application_start.height, |tip| tip.height),
            blocks: &blocks,
            entries: &entries,
        },
        complete,
    )?;
    if Instant::now() >= deadline {
        return Err(refused("feed traversal deadline"));
    }
    Ok(Snapshot { feed, archive })
}
