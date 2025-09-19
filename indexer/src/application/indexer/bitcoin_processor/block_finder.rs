//! Block finder for locating available blocks in the blockchain

use bitcoincore_rpc::bitcoin;

use crate::infrastructure::bitcoin::BitcoinClient;
use crate::utils::logging;

/// Handles finding available blocks in the blockchain
#[derive(Debug)]
pub struct BlockFinder<'a> {
    bitcoin_client: &'a BitcoinClient,
}

impl<'a> BlockFinder<'a> {
    pub fn new(bitcoin_client: &'a BitcoinClient) -> Self {
        Self { bitcoin_client }
    }

    /// Find the first available block starting from a given height using binary search
    pub async fn find_first_available_block(&self, start_height: u64) -> u64 {
        logging::log_info(&format!(
            "[{}] Searching for first available block starting from height {}",
            self.bitcoin_client.network_id().name,
            start_height
        ));

        // Get current chain height to set upper bound
        let chain_height = match self.bitcoin_client.get_block_count().await {
            Ok(height) => height,
            Err(_) => {
                logging::log_error(&format!(
                    "[{}] Could not get chain height, using start height",
                    self.bitcoin_client.network_id().name
                ));
                return start_height;
            }
        };

        // If start height is already at or beyond chain tip, return it
        if start_height >= chain_height {
            return start_height;
        }

        // Use exponential search to find a rough range where blocks become available
        let mut step_size = 1000; // Start with 1000 block jumps
        let mut test_height = start_height;

        // First, try to find any available block by jumping ahead
        while test_height < chain_height {
            if self.is_block_available(test_height).await {
                logging::log_info(&format!(
                    "[{}] Found available block at height {} during exponential search",
                    self.bitcoin_client.network_id().name,
                    test_height
                ));
                break;
            }
            
            test_height += step_size;
            step_size = std::cmp::min(step_size * 2, 10000); // Double step size, max 10k
            
            // Log progress every few jumps
            if (test_height - start_height) % 5000 == 0 {
                logging::log_info(&format!(
                    "[{}] Still searching... checked up to block {}",
                    self.bitcoin_client.network_id().name,
                    test_height
                ));
            }
        }

        // If we reached chain height without finding available blocks, return chain height
        if test_height >= chain_height {
            logging::log_info(&format!(
                "[{}] No available blocks found up to chain height {}, returning chain height",
                self.bitcoin_client.network_id().name,
                chain_height
            ));
            return chain_height;
        }

        // Now use binary search to find the exact first available block
        let mut left = start_height;
        let mut right = test_height;

        while left < right {
            let mid = left + (right - left) / 2;
            
            if self.is_block_available(mid).await {
                right = mid; // Found available block, search left half
            } else {
                left = mid + 1; // Block not available, search right half
            }
        }

        logging::log_info(&format!(
            "[{}] Found first available block at height {} (skipped {} blocks)",
            self.bitcoin_client.network_id().name,
            left,
            left - start_height
        ));

        left
    }

    /// Check if a block is available at the given height
    async fn is_block_available(&self, height: u64) -> bool {
        match self.bitcoin_client.get_block_hash(height).await {
            Ok(block_hash) => self.is_block_data_available(&block_hash).await,
            Err(e) => {
                if self.is_pruned_error(&e) {
                    logging::log_info(&format!(
                        "[{}] Block hash for {} not available (pruned), trying next block",
                        self.bitcoin_client.network_id().name,
                        height
                    ));
                } else {
                    logging::log_error(&format!(
                        "[{}] Unexpected error getting block hash for {}: {}",
                        self.bitcoin_client.network_id().name,
                        height,
                        e
                    ));
                }
                false
            }
        }
    }

    /// Check if block data is available for the given hash
    async fn is_block_data_available(&self, block_hash: &bitcoin::BlockHash) -> bool {
        match self.bitcoin_client.get_block(block_hash).await {
            Ok(_) => true,
            Err(e) => {
                if self.is_pruned_error(&e) {
                    logging::log_info(&format!(
                        "[{}] Block {} not available (pruned)",
                        self.bitcoin_client.network_id().name,
                        block_hash
                    ));
                } else {
                    logging::log_error(&format!(
                        "[{}] Unexpected error getting block {}: {}",
                        self.bitcoin_client.network_id().name,
                        block_hash,
                        e
                    ));
                }
                false
            }
        }
    }

    /// Check if error indicates pruned data
    fn is_pruned_error<E: std::fmt::Display>(&self, error: &E) -> bool {
        let error_str = error.to_string();
        error_str.contains("Block not available") || error_str.contains("pruned")
    }
}
