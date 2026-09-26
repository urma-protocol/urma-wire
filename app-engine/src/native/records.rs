use crate::config::{HISTORY_BLOCKS, START_FILE_BYTES, SYNC_BLOCKS};
use crate::errors::NativeError;
use crate::native::reader::NodeReader;
use crate::native::service::Service;
use crate::projection::{lookup, project, replies};
use urma_runtime::node::Node;
use urma_wire::index::{self, Index};
use wire_app_core::feed::{Feed, Lookup};
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

    pub fn sync(&self) -> Result<Feed, NativeError> {
        let node = self.settings.connect()?;
        let start = self.start(&node)?;
        let report = index::sync(&NodeReader(node), &self.index_path(), start, SYNC_BLOCKS)?;
        Ok(project(
            &Index::load(&self.index_path())?,
            report.complete_to_tip,
        )?)
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
