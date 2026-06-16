//! Block processor: slim orchestrator for processing individual blocks.
//! Each step delegates to a focused module.

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    AddressTransactionsRepository, BlockStatusRepository, MempoolSpendsRepository,
    MonitoredAddressesRepository, ReorgEventsRepository, SummaryRepository,
    TransactionRepository, UtxoRepository,
};
use crate::infrastructure::persistence::Repositories;
use crate::utils::logging;

use super::batch::BatchProcessor;
use super::reorg::{self, ReorgDecision};
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
    reorg_events_repository: ReorgEventsRepository,
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
            reorg_events_repository: repos.reorg_events.clone(),
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

        // STEP -1: Reorg guard. If the previous-block hash doesn't match what
        // we have stored, roll back to the common ancestor and signal the
        // caller to resume from there.
        match reorg::check_and_recover(
            height,
            &block,
            network_id,
            &self.bitcoin_client,
            &self.block_status_repository,
            &self.reorg_events_repository,
        )
        .await?
        {
            ReorgDecision::None => {}
            ReorgDecision::RolledBackTo(h) => {
                return Err(BlockProcessorError::ReorgRolledBackTo(h));
            }
        }

        // STEP 1: Detect charms from all transactions (Strict ZK).
        // Runs BEFORE the mempool consolidator so we know exactly which
        // block txids passed verification — the consolidator then promotes
        // only those mempool rows and purges the rest. Plan 15.
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

        // STEP 0: Consolidate mempool, informed by the verified set.
        let verified_txids: std::collections::HashSet<String> = transaction_batch
            .iter()
            .map(|t| t.txid.clone())
            .collect();
        mempool_consolidator::consolidate(
            &block,
            height,
            network_id,
            &self.mempool_spends_repository,
            &verified_txids,
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

        // STEP 3: Save charms; gather POSITIVE holder deltas (don't apply yet).
        let add_deltas = batch_processor
            .save_charm_batch(charm_batch.clone(), height, network_id)
            .await?;

        // STEP 4: Save assets
        batch_processor
            .save_asset_batch(asset_batch, height, network_id)
            .await?;

        // STEP 5: Mark spent charms; gather NEGATIVE holder deltas (don't apply yet).
        let sub_deltas = spent_tracker::mark_spent_charms(
            &block,
            height,
            network_id,
            &self.charm_service,
            &self.retry_handler,
        )
        .await?;

        // STEP 5.0: Merge add + sub deltas into a single net update per
        // (app_id, address) and apply it once. Splitting the writes across
        // two `update_holders_batch` calls in the same block re-trips the
        // `last_updated_block < block` gate at the repo layer (anomaly A1):
        // the second call always sees `last_updated_block == block` and
        // silently drops the negative delta. Net-then-apply removes the
        // race without weakening the gate's crash-recovery guarantee.
        self.apply_merged_holder_updates(add_deltas, sub_deltas, network_id)
            .await;

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

    /// Merge the additive deltas from `save_charm_batch` with the
    /// subtractive deltas from `mark_spent_charms` by (app_id, address)
    /// and apply a single `update_holders_batch` call. Zero-net entries are
    /// skipped (their balance did not change). `last_updated_block` is the
    /// max height seen on either side so the gate at the repo advances.
    async fn apply_merged_holder_updates(
        &self,
        adds: Vec<(String, String, i64, i32)>,
        subs: Vec<(String, String, i64, i32)>,
        network_id: &NetworkId,
    ) {
        if adds.is_empty() && subs.is_empty() {
            return;
        }
        let mut merged: std::collections::HashMap<(String, String), (i64, i32)> =
            std::collections::HashMap::with_capacity(adds.len() + subs.len());
        for (app_id, address, delta, block_height) in adds.into_iter().chain(subs) {
            let entry = merged
                .entry((app_id, address))
                .or_insert((0i64, block_height));
            entry.0 = entry.0.saturating_add(delta);
            entry.1 = entry.1.max(block_height);
        }
        let updates: Vec<(String, String, i64, i32)> = merged
            .into_iter()
            .filter(|(_, (delta, _))| *delta != 0)
            .map(|((app_id, address), (delta, h))| (app_id, address, delta, h))
            .collect();
        if updates.is_empty() {
            return;
        }
        if let Err(e) = self
            .charm_service
            .get_stats_holders_repository()
            .update_holders_batch(updates, &network_id.name)
            .await
        {
            logging::log_warning(&format!(
                "[{}] Failed to apply merged stats_holders update: {}",
                network_id.name, e
            ));
        }
    }
}
