//! Block processor: slim orchestrator for processing individual blocks.
//! Each step delegates to a focused module.

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    AddressTransactionsRepository, BlockStatusRepository, MempoolSpendsRepository,
    MonitoredAddressesRepository, SummaryRepository, TransactionRepository, UtxoRepository,
};
use crate::infrastructure::persistence::Repositories;
use crate::utils::logging;

use super::batch::BatchProcessor;
use super::retry::RetryHandler;
use super::summary::SummaryUpdater;
use super::{detection, mempool_consolidator, spent_tracker, utxo_indexer};

/// Handles processing of individual blocks
#[derive(Debug)]
pub struct BlockProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    transaction_repository: TransactionRepository,
    summary_repository: SummaryRepository,
    block_status_repository: BlockStatusRepository,
    utxo_repository: UtxoRepository,
    monitored_addresses_repository: MonitoredAddressesRepository,
    mempool_spends_repository: MempoolSpendsRepository,
    address_transactions_repository: AddressTransactionsRepository,
    retry_handler: RetryHandler,
}

impl BlockProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        repos: &Repositories,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            transaction_repository: repos.transaction.clone(),
            summary_repository: repos.summary.clone(),
            block_status_repository: repos.block_status.clone(),
            utxo_repository: repos.utxo.clone(),
            monitored_addresses_repository: repos.monitored_addresses.clone(),
            mempool_spends_repository: repos.mempool_spends.clone(),
            address_transactions_repository: repos.address_transactions.clone(),
            retry_handler: RetryHandler::new(),
        }
    }

    /// Process a single block: detect → save → mark spent → update stats
    #[tracing::instrument(
        name = "block",
        skip_all,
        fields(network = %network_id.name, height),
    )]
    pub async fn process_block(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        let started = std::time::Instant::now();
        let latest_height = self
            .bitcoin_client
            .get_block_count()
            .await
            .map_err(BlockProcessorError::BitcoinClientError)?;

        let block_hash = self
            .bitcoin_client
            .get_block_hash(height)
            .await
            .map_err(BlockProcessorError::BitcoinClientError)?;
        let block = self
            .bitcoin_client
            .get_block(&block_hash)
            .await
            .map_err(BlockProcessorError::BitcoinClientError)?;

        // STEP 0: Promote mempool entries to confirmed
        mempool_consolidator::consolidate(
            &block,
            height,
            network_id,
            &self.mempool_spends_repository,
        )
        .await;

        // STEP 1: Detect charms from all transactions
        let dex_repo = self.charm_service.get_dex_orders_repository();
        let (transaction_batch, charm_batch, asset_batch) = detection::detect_charms(
            &block,
            height,
            latest_height,
            &network_id.name,
            "Bitcoin",
            &self.charm_service,
            Some(dex_repo),
        )
        .await;

        let batch_processor = BatchProcessor::new(
            self.charm_service.clone(),
            self.transaction_repository.clone(),
        );

        // STEP 2: Save transactions
        batch_processor
            .save_transaction_batch(transaction_batch.clone(), height, network_id)
            .await?;

        // STEP 3: Save charms
        batch_processor
            .save_charm_batch(charm_batch.clone(), height, network_id)
            .await?;

        // STEP 4: Save assets
        batch_processor
            .save_asset_batch(asset_batch, height, network_id)
            .await?;

        // STEP 5: Mark spent charms
        spent_tracker::mark_spent_charms(
            &block,
            network_id,
            &self.charm_service,
            &self.retry_handler,
        )
        .await?;

        // STEP 5.5a: Auto-register charm addresses for monitoring
        utxo_indexer::register_charm_addresses(
            &charm_batch,
            network_id,
            &self.monitored_addresses_repository,
        )
        .await;

        // STEP 5.5b: Update UTXO index for monitored addresses
        utxo_indexer::update_monitored_utxos(
            &block,
            height,
            network_id,
            &self.monitored_addresses_repository,
            &self.utxo_repository,
        )
        .await?;

        // STEP 5.5c: Record address transactions for monitored addresses
        utxo_indexer::record_address_transactions(
            &block,
            height,
            network_id,
            &self.monitored_addresses_repository,
            &self.address_transactions_repository,
        )
        .await;

        // STEP 6: Update summary statistics
        let summary_updater =
            SummaryUpdater::new(self.bitcoin_client.clone(), self.summary_repository.clone());
        summary_updater
            .update_statistics(
                height,
                latest_height,
                &charm_batch,
                &transaction_batch,
                network_id,
            )
            .await?;

        // STEP 7: Update block_status
        let confirmations = latest_height.saturating_sub(height) + 1;
        let _ = self
            .block_status_repository
            .mark_downloaded(
                height as i32,
                Some(&block_hash.to_string()),
                Some(&block.header.prev_blockhash.to_string()),
                block.txdata.len() as i32,
                network_id,
            )
            .await;
        let _ = self
            .block_status_repository
            .mark_processed(height as i32, charm_batch.len() as i32, network_id)
            .await;
        if confirmations >= 6 {
            let _ = self
                .block_status_repository
                .mark_confirmed(height as i32, network_id)
                .await;
        }

        let remaining = latest_height.saturating_sub(height);
        logging::log_info(&format!(
            "[{}] ✅ Block {}: Tx {} | Charms {} ({} remaining)",
            network_id.name,
            height,
            block.txdata.len(),
            charm_batch.len(),
            remaining
        ));

        // Metrics: block + per-asset_type charm counters + current height gauge.
        crate::utils::metrics::block_processed(
            &network_id.name,
            started.elapsed().as_secs_f64(),
        );
        crate::utils::metrics::current_height(&network_id.name, height);
        for charm in &charm_batch {
            crate::utils::metrics::charm_detected(&network_id.name, &charm.asset_type);
        }

        Ok(())
    }

    /// Process a block from cached transactions (reindex mode)
    pub async fn process_block_from_cache(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        super::reindex::process_block_from_cache(
            height,
            network_id,
            &self.bitcoin_client,
            &self.charm_service,
            &self.transaction_repository,
            &self.block_status_repository,
            &self.monitored_addresses_repository,
            &self.retry_handler,
        )
        .await
    }
}
