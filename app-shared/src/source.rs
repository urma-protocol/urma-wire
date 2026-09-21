use crate::{
    model::{Chain, ConfirmedStatus, Txid, WireError},
    traits::ChainSource,
};
use std::collections::HashMap;

/// Mock implementation of `ChainSource`.
#[derive(Clone, Debug)]
pub struct MockChainSource {
    chain: Chain,
    tip_height: u64,
    block_hashes: HashMap<u64, String>,
}

impl MockChainSource {
    pub fn new(chain: Chain, start_height: u64) -> Self {
        let mut block_hashes = HashMap::new();
        for h in (start_height.saturating_sub(100))..=start_height {
            block_hashes.insert(h, format!("hash_{}_{}", chain.code(), h));
        }
        Self {
            chain,
            tip_height: start_height,
            block_hashes,
        }
    }

    /// Advance tip by n blocks (simulating incoming blocks).
    pub fn advance_blocks(&mut self, count: u64) {
        for _ in 0..count {
            self.tip_height += 1;
            let hash = format!("hash_{}_{}", self.chain.code(), self.tip_height);
            self.block_hashes.insert(self.tip_height, hash);
        }
    }
}

impl ChainSource for MockChainSource {
    fn chain(&self) -> Chain {
        self.chain
    }

    fn get_tip_height(&self) -> Result<u64, WireError> {
        Ok(self.tip_height)
    }

    fn get_block_hash(&self, height: u64) -> Result<String, WireError> {
        self.block_hashes
            .get(&height)
            .cloned()
            .ok_or_else(|| WireError::NotFound(format!("block at height {}", height)))
    }

    fn check_confirmation(&self, _txid: &Txid) -> Result<Option<ConfirmedStatus>, WireError> {
        // Mock: by default, sample transactions are confirmed
        Ok(Some(ConfirmedStatus {
            block_height: self.tip_height.saturating_sub(5),
            block_hash: format!(
                "hash_{}_{}",
                self.chain.code(),
                self.tip_height.saturating_sub(5)
            ),
            confirmations: 6,
            tx_index: 1,
        }))
    }
}
