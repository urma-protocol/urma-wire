use crate::config::{
    retry_delay_secs, HISTORY_BLOCKS, NETWORK, START_FILE_BYTES, SYNC_BLOCKS, SYNC_RETRY_ATTEMPTS,
};
use crate::errors::NativeError;
use crate::native::archive;
use crate::native::reader::NodeReader;
use crate::native::service::{locked_link, locked_sync, opened, Link, Service};
use crate::projection::{lookup, project, replies};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use urma_runtime::node::Node;
use urma_wire::index::{self, Index};
use wire_app_core::connection::SourceChoice;
use wire_app_core::feed::{Feed, Lookup, SyncPhase, SyncStatus};
use wire_app_core::model::{Placement, Post, Txid};

impl Service {
    pub(super) fn start(&self, node: &Node) -> Result<u64, NativeError> {
        start_at(&self.directory.join(Self::START_FILE), node)
    }

    pub fn start_sync(&self) -> Result<SyncStatus, NativeError> {
        let mut status = self.sync_status();
        if !status.settled() {
            return Ok(status.clone());
        }
        let settings = self.settings().clone();
        if let Some(configured) = settings.archive.clone() {
            status.run += 1;
            status.phase = SyncPhase::Connecting;
            status.complete_to_tip = false;
            status.done = 0;
            status.total = 0;
            let snapshot = status.clone();
            drop(status);
            let shared_status = self.sync.clone();
            let feed = self.archive_feed.clone();
            let link = self.link.clone();
            let directory = self.directory.clone();
            let providers = self.local_node.providers();
            let spawned = std::thread::Builder::new().name("wire-archive".into()).spawn(move || {
                match archive::read(&configured) {
                    Ok(snapshot) => {
                        let mut status = locked_sync(&shared_status);
                        status.start = snapshot.feed.start;
                        status.from = snapshot.feed.start;
                        status.tip = snapshot.feed.tip;
                        status.indexed_to = snapshot.archive.indexed_through.map(|point| point.height);
                        status.complete_to_tip = snapshot.feed.complete_to_tip;
                        // These are coverage counts, not the number of sparse
                        // placement checkpoints or selectively returned records.
                        status.done = snapshot.archive.scanned_blocks;
                        status.total = snapshot.archive.selected_tip.map_or(0, |tip|
                            tip.height.saturating_sub(snapshot.feed.start).saturating_add(1));
                        *feed.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.feed);
                        status.phase = SyncPhase::Complete;
                    }
                    Err(error) => {
                        tracing::info!(error = %error, "optional archive unavailable; using autonomous history sync");
                        *feed.lock().unwrap_or_else(|error| error.into_inner()) = None;
                        let fallback = (|| -> Result<SyncWorker, NativeError> {
                            let node = {
                                let mut link = locked_link(&link);
                                match &*link {
                                    Link::Open(node) => node.clone(),
                                    Link::Closed => {
                                        let node = if settings.source == SourceChoice::Public {
                                            Node::public_in_with_providers(NETWORK, &settings.cache_dir, providers)?
                                        } else { settings.connect()? };
                                        let node = Arc::new(node);
                                        opened(&node);
                                        *link = Link::Open(node.clone());
                                        node
                                    }
                                }
                            };
                            let start = start_at(&directory.join(Service::START_FILE), &node)?;
                            {
                                let mut status = locked_sync(&shared_status);
                                status.start = start;
                                status.from = start;
                            }
                            Ok(SyncWorker { node, path: directory.join(Service::INDEX_FILE),
                                start, status: shared_status.clone() })
                        })();
                        match fallback {
                            Ok(worker) => worker.run(),
                            Err(error) => locked_sync(&shared_status).phase = SyncPhase::Failed(error.to_string()),
                        }
                    }
                }
            });
            if let Err(error) = spawned {
                self.sync_status().phase = SyncPhase::Failed(error.to_string());
                return Err(error.into());
            }
            return Ok(snapshot);
        }
        *self
            .archive_feed
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        let node = self.node()?;
        let start = self.start(&node)?;
        *status = SyncStatus {
            run: status.run + 1,
            phase: SyncPhase::Connecting,
            start,
            from: start,
            indexed_to: status.indexed_to,
            tip: status.tip,
            done: 0,
            total: 0,
            complete_to_tip: false,
        };
        let snapshot = status.clone();
        drop(status);
        let worker = SyncWorker {
            node,
            path: self.index_path(),
            start,
            status: self.sync.clone(),
        };
        std::thread::Builder::new()
            .name("wire-sync".into())
            .spawn(move || worker.run())?;
        Ok(snapshot)
    }

    pub fn sync_snapshot(&self) -> SyncStatus {
        self.sync_status().clone()
    }

    pub fn feed(&self) -> Result<Feed, NativeError> {
        // Match the worker's publication lock order. The coverage flag and
        // cached feed must belong to the same publication, even during refresh.
        let status = {
            let status = self.sync_status();
            let cached = self
                .archive_feed
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(mut feed) = cached {
                feed.complete_to_tip &= status.complete_to_tip;
                return Ok(feed);
            }
            status.clone()
        };
        let index = match self.index_path().exists() {
            true => Index::load(&self.index_path())?,
            false => Index {
                format: Index::FORMAT.into(),
                genesis: NETWORK.genesis()?.0.to_string(),
                start: status.start,
                blocks: Vec::new(),
                entries: Vec::new(),
            },
        };
        Ok(project(&index, status.complete_to_tip)?)
    }

    pub fn post(&self, txid: Txid) -> Result<Lookup, NativeError> {
        if let Some(feed) = self
            .archive_feed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            return Ok(feed
                .posts
                .iter()
                .find(|post| post.txid == txid)
                .cloned()
                .map_or(Lookup::Absent, Lookup::Found));
        }
        if !self.index_path().exists() {
            return Ok(Lookup::Absent);
        }
        Ok(lookup(&Index::load(&self.index_path())?, txid)?)
    }

    pub fn replies(&self, txid: Txid) -> Result<Vec<Post>, NativeError> {
        if let Some(feed) = self
            .archive_feed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            return Ok(feed
                .posts
                .iter()
                .filter(|post| post.placement == Placement::Reply(txid))
                .cloned()
                .collect());
        }
        if !self.index_path().exists() {
            return Ok(Vec::new());
        }
        Ok(replies(&Index::load(&self.index_path())?, txid)?)
    }
}

fn start_at(path: &Path, node: &Node) -> Result<u64, NativeError> {
    if path.try_exists()? {
        return Ok(serde_json::from_slice(&urma_io::read_bounded(
            path,
            START_FILE_BYTES,
        )?)?);
    }
    let tip = node.tip_height()?;
    let start = tip.max(HISTORY_BLOCKS) - HISTORY_BLOCKS;
    urma_io::write_new(path, &serde_json::to_vec(&start)?)?;
    Ok(start)
}

struct SyncWorker {
    node: Arc<Node>,
    path: PathBuf,
    start: u64,
    status: Arc<Mutex<SyncStatus>>,
}

enum Batch {
    Done,
    More,
}

impl SyncWorker {
    fn run(self) {
        let mut attempt: u32 = 0;
        loop {
            match self.batch() {
                Ok(Batch::Done) => return,
                Ok(Batch::More) => attempt = 0,
                Err(error) => {
                    attempt += 1;
                    if attempt > SYNC_RETRY_ATTEMPTS {
                        tracing::warn!(error = %error, attempt, "history sync gave up");
                        locked_sync(&self.status).phase = SyncPhase::Failed(format!(
                            "Sync gave up after {SYNC_RETRY_ATTEMPTS} retries: {error}"
                        ));
                        return;
                    }
                    tracing::warn!(error = %error, attempt, "history sync batch failed; retrying");
                    self.wait(attempt, error.to_string());
                }
            }
        }
    }

    fn batch(&self) -> Result<Batch, NativeError> {
        let reader = NodeReader(&self.node);
        let (report, index) = index::sync(
            &reader,
            &self.path,
            self.start,
            SYNC_BLOCKS,
            |height, tip| {
                let mut status = locked_sync(&self.status);
                status.from = match status.phase {
                    SyncPhase::Verifying => status.from,
                    SyncPhase::Idle
                    | SyncPhase::Connecting
                    | SyncPhase::Retrying { .. }
                    | SyncPhase::Complete
                    | SyncPhase::Failed(_) => height,
                };
                status.phase = SyncPhase::Verifying;
                status.tip = tip;
                status.indexed_to = Some(height);
                status.done = height - status.from + 1;
                status.total = tip.max(height) - status.from + 1;
            },
        )?;
        let mut status = locked_sync(&self.status);
        status.tip = report.tip;
        status.indexed_to = index.blocks.last().map(|block| block.height);
        status.complete_to_tip = report.complete_to_tip;
        if report.complete_to_tip || report.scanned == 0 {
            status.done = status.total;
            status.phase = SyncPhase::Complete;
            tracing::info!(
                tip = report.tip,
                records = report.records,
                "history sync complete"
            );
            return Ok(Batch::Done);
        }
        Ok(Batch::More)
    }

    fn wait(&self, attempt: u32, reason: String) {
        for seconds in (1..=retry_delay_secs(attempt)).rev() {
            locked_sync(&self.status).phase = SyncPhase::Retrying {
                attempt,
                seconds,
                reason: reason.clone(),
            };
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
