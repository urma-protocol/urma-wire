use crate::config::{ArchiveSettings, NETWORK};
use crate::errors::NativeError;
use crate::projection::{project_view, View};
use bitcoin::BlockHash;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};
use urma_node_client::model::{ArchiveStatus, ChainPoint, SCAN_RULES_VERSION};
use urma_node_client::projection::{IndexSnapshot, WireCursor, WirePage};
use urma_node_client::{Client, Error as ClientError};
use urma_wire::index::{Checkpoint, Entry};
use wire_app_core::feed::{Feed, FeedSource};
use zeroize::Zeroizing;

pub(super) enum OperatorState {
    Held(OperatorView),
    Absent,
}

pub(super) struct OperatorView {
    pub feed: Feed,
    pub archive: ArchiveStatus,
    pub acquired: Instant,
}

impl OperatorView {
    const MAX_RECORDS: usize = 50_000;
    const MAX_RECORD_HEX: usize = 64 * 1024 * 1024;
    const TRAVERSAL_TIME: Duration = Duration::from_secs(30);
    const PAGE: u32 = 128;
    const TOKEN_BYTES: usize = 256;
    const TOKEN_READ: u64 = 257;
    const MAX_LAG: u64 = 2;
    const MAX_AGE: Duration = Duration::from_secs(600);

    pub(super) fn fresh(&self) -> bool {
        self.acquired.elapsed() < Self::MAX_AGE
    }
}

struct First {
    snapshot: IndexSnapshot,
    encoded: Vec<u8>,
    archive: ArchiveStatus,
}

struct Traversal {
    first: First,
    entries: Vec<Entry>,
    placements: BTreeMap<u64, BlockHash>,
    hex_bytes: usize,
}

fn refused(message: impl Into<String>) -> NativeError {
    NativeError::Archive(message.into())
}

fn unavailable(error: ClientError) -> NativeError {
    match error {
        ClientError::Io(io) => refused(format!("feed I/O: {}", io.kind())),
        ClientError::Json(json) => refused(format!("feed encoding: {:?}", json.classify())),
        ClientError::Invalid(message) => refused(format!(
            "feed refused; {} byte diagnostic withheld",
            message.len()
        )),
        ClientError::Busy(message) => refused(format!(
            "feed busy or snapshot changed; {} byte diagnostic withheld",
            message.len()
        )),
    }
}

fn before(deadline: Instant) -> Result<(), NativeError> {
    match Instant::now() < deadline {
        true => Ok(()),
        false => Err(refused("feed traversal deadline")),
    }
}

pub(super) fn read(settings: &ArchiveSettings) -> Result<OperatorView, NativeError> {
    let deadline = Instant::now() + OperatorView::TRAVERSAL_TIME;
    let token = bearer(&settings.token_file)?;
    let client = Client::new(settings.endpoint, token.as_str().to_owned()).map_err(unavailable)?;
    let page = fetch(&client, None, deadline)?;
    let mut traversal = Traversal::begin(&page)?;
    let mut next = traversal.admit(page)?;
    loop {
        let Some(cursor) = next else {
            break;
        };
        next = traversal.admit(fetch(&client, Some(&cursor), deadline)?)?;
    }
    let view = traversal.finish()?;
    before(deadline)?;
    Ok(view)
}

fn fetch(
    client: &Client,
    cursor: Option<&WireCursor>,
    deadline: Instant,
) -> Result<WirePage, NativeError> {
    before(deadline)?;
    let page = client
        .wire_page(cursor, OperatorView::PAGE)
        .map_err(unavailable)?;
    before(deadline)?;
    Ok(page)
}

impl Traversal {
    fn begin(page: &WirePage) -> Result<Self, NativeError> {
        Ok(Self {
            first: First {
                snapshot: page.snapshot.clone(),
                encoded: serde_json::to_vec(&page.archive)?,
                archive: page.archive.clone(),
            },
            entries: Vec::new(),
            placements: BTreeMap::new(),
            hex_bytes: 0,
        })
    }

    fn admit(&mut self, page: WirePage) -> Result<Option<WireCursor>, NativeError> {
        if page.archive.chain_genesis != NETWORK.genesis()?.0
            || page.archive.scan_rules_version != SCAN_RULES_VERSION
        {
            return Err(refused("wrong chain or scan rules"));
        }
        if self.first.snapshot != page.snapshot
            || self.first.encoded != serde_json::to_vec(&page.archive)?
        {
            return Err(refused("archive snapshot changed within traversal"));
        }
        if self.entries.len() + page.records.len() > OperatorView::MAX_RECORDS {
            return Err(refused("feed record capacity exceeded"));
        }
        for record in page.records {
            self.hex_bytes += record.entry.record.len();
            if self.hex_bytes > OperatorView::MAX_RECORD_HEX {
                return Err(refused("feed byte capacity exceeded"));
            }
            let earlier = self
                .placements
                .insert(record.entry.height, record.block_hash);
            for placed in earlier.iter() {
                if *placed != record.block_hash {
                    return Err(refused("conflicting placement across pages"));
                }
            }
            self.entries.push(record.entry);
        }
        Ok(page.next)
    }

    fn finish(self) -> Result<OperatorView, NativeError> {
        let archive = self.first.archive;
        let (indexed, tip) = covered(&archive)?;
        let blocks: Vec<Checkpoint> = self
            .placements
            .into_iter()
            .map(|(height, hash)| Checkpoint {
                height,
                hash: hash.to_string(),
            })
            .collect();
        let mut feed = project_view(
            &View {
                start: archive.application_start.height,
                tip: tip.height,
                blocks: &blocks,
                entries: &self.entries,
            },
            indexed == tip,
        )?;
        feed.source = FeedSource::LocalNodeOperator {
            indexed_through: Some(indexed.height),
            peer_synced_through: archive.peer_synced_through.map(|point| point.height),
            generation: archive.generation,
            trust: FeedSource::OPERATOR_TRUST.into(),
        };
        Ok(OperatorView {
            feed,
            archive,
            acquired: Instant::now(),
        })
    }
}

fn covered(archive: &ArchiveStatus) -> Result<(ChainPoint, ChainPoint), NativeError> {
    let Some(indexed) = archive.indexed_through else {
        return Err(refused("operator archive has nothing indexed"));
    };
    let Some(tip) = archive.selected_tip else {
        return Err(refused("coverage without selected headers"));
    };
    let start = archive.application_start;
    if indexed.height < start.height
        || indexed.height > tip.height
        || (indexed.height == tip.height && indexed.hash != tip.hash)
        || (indexed.height == start.height && indexed.hash != start.hash)
    {
        return Err(refused("inconsistent archive coverage"));
    }
    let behind = tip.height - indexed.height;
    if behind > OperatorView::MAX_LAG {
        return Err(refused(format!(
            "operator projection {behind} blocks behind its selected tip"
        )));
    }
    Ok((indexed, tip))
}

#[cfg(unix)]
fn bearer(path: &Path) -> Result<Zeroizing<String>, NativeError> {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let linked = std::fs::symlink_metadata(path)?;
    if !linked.is_file() {
        return Err(refused("token must be one private regular file"));
    }
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    let same = opened.is_file() && linked.dev() == opened.dev() && linked.ino() == opened.ino();
    if !same || opened.nlink() != 1 || opened.mode() & 0o077 != 0 {
        return Err(refused("token must be one private regular file"));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(OperatorView::TOKEN_READ)
        .read_to_end(&mut bytes)?;
    if bytes.len() > OperatorView::TOKEN_BYTES {
        return Err(refused("token exceeds its size bound"));
    }
    utf8(&bytes)
}

#[cfg(not(unix))]
fn bearer(path: &Path) -> Result<Zeroizing<String>, NativeError> {
    utf8(&urma_io::read_private(path, OperatorView::TOKEN_BYTES)?)
}

fn utf8(bytes: &[u8]) -> Result<Zeroizing<String>, NativeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        refused(format!(
            "token is not UTF-8 after byte {}",
            error.valid_up_to()
        ))
    })?;
    Ok(Zeroizing::new(text.trim().to_owned()))
}
