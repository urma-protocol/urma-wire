use crate::config::{bounded_slice, HISTORY_BLOCKS, START_FILE_BYTES, SYNC_BLOCKS};
use crate::errors::NativeError;
use crate::native::reader::NodeReader;
use crate::native::service::Service;
use crate::projection::{lookup, project, replies};
use urma_runtime::node::Node;
use urma_wire::index::{self, Index, SyncReport};
use wire_app_core::feed::{Feed, Lookup, SyncProgress};
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

    fn scan(&self, max_blocks: u64) -> Result<(SyncReport, Index), NativeError> {
        let report = self.with_node(|node| {
            let start = self.start(node)?;
            Ok(index::sync(
                &NodeReader(node),
                &self.index_path(),
                start,
                max_blocks,
            )?)
        })?;
        Ok((report, Index::load(&self.index_path())?))
    }

    pub fn sync(&self) -> Result<Feed, NativeError> {
        let (report, index) = self.scan(SYNC_BLOCKS)?;
        Ok(project(&index, report.complete_to_tip)?)
    }

    pub fn sync_slice(&self, max_blocks: u64) -> Result<SyncProgress, NativeError> {
        let (report, index) = self.scan(bounded_slice(max_blocks))?;
        Ok(SyncProgress {
            start: index.start,
            indexed_to: index.blocks.last().map(|block| block.height),
            tip: report.tip,
            scanned: report.scanned,
            records: u64::try_from(report.records)?,
            rolled_back: report.rolled_back,
            complete_to_tip: report.complete_to_tip,
        })
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
