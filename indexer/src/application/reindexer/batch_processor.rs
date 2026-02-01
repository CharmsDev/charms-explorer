//! Batch Processor - Orchestrates reindexing using block_parser and stats_updater modules

use super::{
    block_parser, stats_updater,
    types::{AssetBatch, CharmBatch, SpellBatch},
};
use crate::config::{NetworkId, NetworkType};
use crate::domain::errors::BlockProcessorError;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, BookmarkRepository, CharmRepository, SpellRepository, StatsHoldersRepository,
    TransactionRepository,
};
use crate::utils::logging;
use std::time::Instant;

const LOG_INTERVAL: usize = 100;

/// Batch reindexer for a single network
pub struct BatchReindexer {
    network: String,
    network_id: NetworkId,
    charm_repository: CharmRepository,
    asset_repository: AssetRepository,
    spell_repository: SpellRepository,
    stats_holders_repository: StatsHoldersRepository,
    transaction_repository: TransactionRepository,
    bookmark_repository: BookmarkRepository,
}

impl BatchReindexer {
    pub fn new(
        network: &str,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
        spell_repository: SpellRepository,
        stats_holders_repository: StatsHoldersRepository,
        transaction_repository: TransactionRepository,
        bookmark_repository: BookmarkRepository,
    ) -> Self {
        Self {
            network: network.to_string(),
            network_id: NetworkId::new(NetworkType::Bitcoin, network),
            charm_repository,
            asset_repository,
            spell_repository,
            stats_holders_repository,
            transaction_repository,
            bookmark_repository,
        }
    }

    pub async fn run(&self) -> Result<Option<u64>, BlockProcessorError> {
        let (start, end) = self.get_block_range().await?;
        if start >= end {
            logging::log_info(&format!("[{}] Already at block {}.", self.network, end));
            return Ok(Some(end));
        }
        let blocks = self.get_blocks_to_process(start).await?;
        logging::log_info(&format!(
            "[{}] Reindexing {} → {} ({} blocks)",
            self.network,
            start,
            end,
            blocks.len()
        ));
        Ok(Some(self.process_blocks(&blocks).await))
    }

    async fn get_block_range(&self) -> Result<(u64, u64), BlockProcessorError> {
        let start = self
            .bookmark_repository
            .get_last_processed_block(&self.network_id)
            .await
            .map_err(BlockProcessorError::DbError)?
            .unwrap_or(0);
        let (_, max) = self
            .transaction_repository
            .get_block_range(&self.network)
            .await
            .map_err(BlockProcessorError::DbError)?;
        Ok((start, max.unwrap_or(start)))
    }

    async fn get_blocks_to_process(&self, start: u64) -> Result<Vec<u64>, BlockProcessorError> {
        let all = self
            .transaction_repository
            .get_blocks_with_transactions(&self.network)
            .await
            .map_err(BlockProcessorError::DbError)?;
        Ok(all.into_iter().filter(|h| *h >= start).collect())
    }

    async fn process_blocks(&self, blocks: &[u64]) -> u64 {
        let t0 = Instant::now();
        let (mut n, mut ch, mut last) = (0usize, 0usize, 0u64);
        for (i, &h) in blocks.iter().enumerate() {
            let t = Instant::now();
            match self.process_block_batch(h).await {
                Ok((tx, c)) => {
                    n += 1;
                    ch += c;
                    last = h;
                    let _ = self
                        .bookmark_repository
                        .update_bookmark_height(h, &self.network_id)
                        .await;
                    if i % LOG_INTERVAL == 0 || c > 0 {
                        let bps = n as f64 / t0.elapsed().as_secs_f64().max(0.001);
                        logging::log_info(&format!(
                            "[{}] {} | {}tx {}ch | {:.1}b/s | {}ms",
                            self.network,
                            h,
                            tx,
                            c,
                            bps,
                            t.elapsed().as_millis()
                        ));
                    }
                }
                Err(e) => logging::log_error(&format!("[{}] {}: {}", self.network, h, e)),
            }
        }
        logging::log_info(&format!(
            "[{}] ✓ {} blocks, {} charms in {:.1}s",
            self.network,
            n,
            ch,
            t0.elapsed().as_secs_f64()
        ));
        last
    }

    async fn process_block_batch(&self, h: u64) -> Result<(usize, usize), BlockProcessorError> {
        let txs = self
            .transaction_repository
            .get_transactions_for_reindex(h, &self.network)
            .await
            .map_err(BlockProcessorError::DbError)?;
        if txs.is_empty() {
            return Ok((0, 0));
        }

        let tx_hexes: Vec<String> = txs.iter().map(|(_, hex, _)| hex.clone()).collect();
        let (spells, charms, assets) =
            block_parser::parse_transactions(txs, h, &self.network).await;
        let charm_count = charms.len();

        self.save_batches(spells, charms, assets, h).await;
        let spent = block_parser::extract_spent_txids(&tx_hexes);
        stats_updater::update_spent_holders(
            &self.charm_repository,
            &self.stats_holders_repository,
            spent,
            h,
            &self.network,
        )
        .await;
        stats_updater::update_new_holders(
            &self.charm_repository,
            &self.stats_holders_repository,
            h,
            &self.network,
        )
        .await;
        Ok((tx_hexes.len(), charm_count))
    }

    async fn save_batches(
        &self,
        spells: SpellBatch,
        charms: CharmBatch,
        assets: AssetBatch,
        h: u64,
    ) {
        if !spells.is_empty() {
            if let Err(e) = self.spell_repository.save_batch(spells).await {
                logging::log_warning(&format!("[{}] {}: Spell err: {}", self.network, h, e));
            }
        }
        if !charms.is_empty() {
            if let Err(e) = self.charm_repository.save_batch(charms).await {
                logging::log_warning(&format!("[{}] {}: Charm err: {}", self.network, h, e));
            }
        }
        if !assets.is_empty() {
            if let Err(e) = self.asset_repository.save_batch(assets).await {
                logging::log_warning(&format!("[{}] {}: Asset err: {}", self.network, h, e));
            }
        }
    }
}
