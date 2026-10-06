use crate::config::{HISTORY_BLOCKS, NETWORK, START_FILE_BYTES, SYNC_BLOCKS};
use crate::errors::NativeError;
use crate::native::reader::NodeReader;
use crate::native::service::{drop_link, locked_sync, transport_error, Link, Service};
use crate::projection::{lookup, project, replies};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use urma_runtime::node::Node;
use urma_wire::index::{self, Index};
use wire_app_core::feed::{Feed, Lookup, SyncPhase, SyncStatus};
use wire_app_core::model::{Post, Txid};

impl Service {
    pub(super) fn start(&self, node: &Node) -> Result<u64, NativeError> {
        let path = self.directory.join(Self::START_FILE);
        if path.try_exists()? {
            return Ok(serde_json::from_slice(&urma_io::read_bounded(
                &path,
                START_FILE_BYTES,
            )?)?);
        }
        let tip = node.tip_height()?;
        let start = tip.max(HISTORY_BLOCKS) - HISTORY_BLOCKS;
        urma_io::write_new(&path, &serde_json::to_vec(&start)?)?;
        Ok(start)
    }

    pub fn start_sync(&self) -> Result<SyncStatus, NativeError> {
        let mut status = self.sync_status();
        if !status.settled() {
            return Ok(status.clone());
        }
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
            link: self.link_handle(),
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
        let status = self.sync_snapshot();
        let index = match self.index_path().exists() {
            true => Index::load(&self.index_path())?,
            false => Index {
                format: "URMA-WIRE-INDEX-2".into(),
                genesis: NETWORK.genesis()?.0.to_string(),
                start: status.start,
                blocks: Vec::new(),
                entries: Vec::new(),
            },
        };
        Ok(project(&index, status.complete_to_tip)?)
    }

    pub fn post(&self, txid: Txid) -> Result<Lookup, NativeError> {
        if !self.index_path().exists() {
            return Ok(Lookup::Absent);
        }
        Ok(lookup(&Index::load(&self.index_path())?, txid)?)
    }

    pub fn replies(&self, txid: Txid) -> Result<Vec<Post>, NativeError> {
        if !self.index_path().exists() {
            return Ok(Vec::new());
        }
        Ok(replies(&Index::load(&self.index_path())?, txid)?)
    }
}

struct SyncWorker {
    node: Arc<Node>,
    link: Arc<Mutex<Link>>,
    path: PathBuf,
    start: u64,
    status: Arc<Mutex<SyncStatus>>,
}

impl SyncWorker {
    fn run(self) {
        loop {
            let reader = NodeReader(&self.node);
            let outcome = index::sync(&reader, &self.path, self.start, SYNC_BLOCKS, |height, tip| {
                let mut status = locked_sync(&self.status);
                status.from = match status.phase {
                    SyncPhase::Verifying => status.from,
                    SyncPhase::Idle
                    | SyncPhase::Connecting
                    | SyncPhase::Complete
                    | SyncPhase::Failed(_) => height,
                };
                status.phase = SyncPhase::Verifying;
                status.tip = tip;
                status.indexed_to = Some(height);
                status.done = height - status.from + 1;
                status.total = tip.max(height) - status.from + 1;
            });
            match outcome {
                Ok((report, index)) => {
                    let mut status = locked_sync(&self.status);
                    status.tip = report.tip;
                    status.indexed_to = index.blocks.last().map(|block| block.height);
                    status.complete_to_tip = report.complete_to_tip;
                    if report.complete_to_tip || report.scanned == 0 {
                        status.done = status.total;
                        status.phase = SyncPhase::Complete;
                        tracing::info!(tip = report.tip, records = report.records, "history sync complete");
                        return;
                    }
                }
                Err(error) => {
                    let error = NativeError::from(error);
                    tracing::warn!(error = %error, "history sync stopped");
                    if transport_error(&error) {
                        drop_link(&self.link, &self.node);
                    }
                    locked_sync(&self.status).phase = SyncPhase::Failed(error.to_string());
                    return;
                }
            }
        }
    }
}
