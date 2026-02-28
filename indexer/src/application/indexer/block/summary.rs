//! Summary statistics updater for maintaining the summary table

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::SummaryRepository;

use super::batch::{CharmBatchItem, TransactionBatchItem};
use super::retry::RetryHandler;

/// Handles updating summary statistics after block processing
#[derive(Debug)]
pub struct SummaryUpdater {
    bitcoin_client: BitcoinClient,
    summary_repository: SummaryRepository,
    retry_handler: RetryHandler,
}

impl SummaryUpdater {
    pub fn new(bitcoin_client: BitcoinClient, summary_repository: SummaryRepository) -> Self {
        Self {
            bitcoin_client,
            summary_repository,
            retry_handler: RetryHandler::new(),
        }
    }

    /// Update summary statistics after processing a block
    pub async fn update_statistics(
        &self,
        height: u64,
        latest_height: u64,
        charm_batch: &[CharmBatchItem],
        transaction_batch: &[TransactionBatchItem],
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        let (bitcoin_node_status, bitcoin_node_block_count, bitcoin_node_best_block_hash) =
            match self.bitcoin_client.get_best_block_hash().await {
                Ok(best_hash) => (
                    "connected".to_string(),
                    latest_height as i64,
                    best_hash.to_string(),
                ),
                Err(_) => ("error".to_string(), 0i64, "unknown".to_string()),
            };

        let latest_confirmed_block = if latest_height >= 6 {
            latest_height - 5
        } else {
            0
        };

        let asset_counts = calculate_asset_counts(charm_batch);

        let confirmed_transactions = transaction_batch
            .iter()
            .filter(|tx| tx.6)
            .count() as i64;

        let current_summary = self
            .summary_repository
            .get_summary(network_id)
            .await
            .map_err(BlockProcessorError::DbError)?;

        let totals = calculate_totals(
            current_summary,
            charm_batch,
            transaction_batch,
            confirmed_transactions,
            &asset_counts,
        );

        self.retry_handler
            .execute_with_retry_and_logging(
                || async {
                    self.summary_repository
                        .update_summary_with_bitcoin_node(
                            network_id,
                            height as i32,
                            latest_confirmed_block as i32,
                            totals.total_charms,
                            totals.total_transactions,
                            totals.total_confirmed_transactions,
                            totals.total_nft_count,
                            totals.total_token_count,
                            totals.total_dapp_count,
                            totals.total_other_count,
                            Some(bitcoin_node_status.clone()),
                            Some(bitcoin_node_block_count),
                            Some(bitcoin_node_best_block_hash.clone()),
                        )
                        .await
                },
                "update summary statistics",
                &network_id.name,
            )
            .await
            .map_err(BlockProcessorError::DbError)?;

        Ok(())
    }
}

fn calculate_asset_counts(charm_batch: &[CharmBatchItem]) -> AssetCounts {
    let mut counts = AssetCounts::default();
    for charm_item in charm_batch {
        match charm_item.4.as_str() {
            "nft" => counts.nft_count += 1,
            "token" => counts.token_count += 1,
            _ => counts.other_count += 1,
        }
    }
    counts
}

fn calculate_totals(
    current_summary: Option<crate::infrastructure::persistence::entities::summary::Model>,
    charm_batch: &[CharmBatchItem],
    transaction_batch: &[TransactionBatchItem],
    confirmed_transactions: i64,
    asset_counts: &AssetCounts,
) -> SummaryTotals {
    if let Some(summary) = current_summary {
        SummaryTotals {
            total_charms: summary.total_charms + charm_batch.len() as i64,
            total_transactions: summary.total_transactions + transaction_batch.len() as i64,
            total_confirmed_transactions: summary.confirmed_transactions + confirmed_transactions,
            total_nft_count: summary.nft_count + asset_counts.nft_count,
            total_token_count: summary.token_count + asset_counts.token_count,
            total_dapp_count: summary.dapp_count + asset_counts.dapp_count,
            total_other_count: summary.other_count + asset_counts.other_count,
        }
    } else {
        SummaryTotals {
            total_charms: charm_batch.len() as i64,
            total_transactions: transaction_batch.len() as i64,
            total_confirmed_transactions: confirmed_transactions,
            total_nft_count: asset_counts.nft_count,
            total_token_count: asset_counts.token_count,
            total_dapp_count: asset_counts.dapp_count,
            total_other_count: asset_counts.other_count,
        }
    }
}

#[derive(Default)]
struct AssetCounts {
    nft_count: i64,
    token_count: i64,
    dapp_count: i64,
    other_count: i64,
}

struct SummaryTotals {
    total_charms: i64,
    total_transactions: i64,
    total_confirmed_transactions: i64,
    total_nft_count: i64,
    total_token_count: i64,
    total_dapp_count: i64,
    total_other_count: i64,
}
