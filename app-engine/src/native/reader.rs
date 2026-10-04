use bitcoin::{Block, BlockHash, Transaction};
use urma_runtime::{error::Error, node::Node};
use urma_wire::reader::Reader;

pub struct NodeReader<'a>(pub &'a Node);

impl Reader for NodeReader<'_> {
    type Error = Error;

    fn genesis(&self) -> Result<BlockHash, Error> {
        Ok(self.0.chain().genesis()?.0)
    }

    fn tip_height(&self) -> Result<u64, Error> {
        self.0.tip_height()
    }

    fn block_hash(&self, height: u64) -> Result<BlockHash, Error> {
        self.0.block_hash(height)
    }

    fn block(&self, height: u64) -> Result<(Block, BlockHash), Error> {
        self.0.block_at(height)
    }

    fn transaction(&self, txid: bitcoin::Txid) -> Result<Transaction, Error> {
        self.0.transaction(txid)
    }
}
