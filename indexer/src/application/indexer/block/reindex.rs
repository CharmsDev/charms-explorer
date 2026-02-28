//! Reindex path: process blocks from cached transactions in the database.

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::tx_analyzer;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    BlockStatusRepository, MonitoredAddressesRepository, TransactionRepository,
};
use crate::utils::logging;

use super::retry::RetryHandler;
use super::spent_tracker;

/// Process a block from cached transactions in database (reindex mode).
/// Uses data from transactions table instead of fetching from Bitcoin node.
pub async fn process_block_from_cache(
    height: u64,
    network_id: &NetworkId,
    bitcoin_client: &BitcoinClient,
    charm_service: &CharmService,
    transaction_repository: &TransactionRepository,
    block_status_repository: &BlockStatusRepository,
    monitored_addresses_repository: &MonitoredAddressesRepository,
    retry_handler: &RetryHandler,
) -> Result<(), BlockProcessorError> {
    let cached_txs = transaction_repository
        .find_by_block_height(height)
        .await
        .map_err(|e| BlockProcessorError::ProcessingError(format!("DB error: {}", e)))?;

    if cached_txs.is_empty() {
        let _ = block_status_repository
            .mark_processed(height as i32, 0, network_id)
            .await;
        return Ok(());
    }

    let network = network_id.name.clone();
    let blockchain = "Bitcoin".to_string();

    let mut charm_count = 0;
    let mut charm_addresses: Vec<String> = Vec::new();

    for tx in &cached_txs {
        let tx_hex = tx.raw.get("hex").and_then(|v| v.as_str()).unwrap_or("");
        if tx_hex.is_empty() {
            continue;
        }

        let analyzed = match tx_analyzer::analyze_tx(&tx.txid, tx_hex, &network) {
            Some(a) => a,
            None => continue,
        };

        charm_count += 1;
        if let Some(addr) = &analyzed.address {
            if !addr.is_empty() {
                charm_addresses.push(addr.clone());
            }
        }

        if let Err(e) = charm_service
            .save_batch(vec![(
                tx.txid.clone(),
                0i32,
                height,
                analyzed.charm_json.clone(),
                analyzed.asset_type.clone(),
                blockchain.clone(),
                network.clone(),
                analyzed.address.clone(),
                analyzed.app_id.clone(),
                analyzed.amount,
                analyzed.tags.clone(),
            )])
            .await
        {
            logging::log_error(&format!(
                "[{}] Error saving charm for tx {} at height {}: {}",
                network_id.name, tx.txid, height, e
            ));
        }
    }

    // Auto-register charm addresses for monitoring
    if !charm_addresses.is_empty() {
        let unique: Vec<String> = charm_addresses
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if let Ok(n) = monitored_addresses_repository
            .register_batch(&unique, &network_id.name, "indexer")
            .await
        {
            if n > 0 {
                logging::log_info(&format!(
                    "[{}] 📡 Reindex: registered {} new monitored addresses",
                    network_id.name, n
                ));
            }
        }
    }

    // Mark spent charms by fetching the full block from the node
    match bitcoin_client.get_block_hash(height).await {
        Ok(block_hash) => match bitcoin_client.get_block(&block_hash).await {
            Ok(block) => {
                if let Err(e) =
                    spent_tracker::mark_spent_charms(&block, network_id, charm_service, retry_handler)
                        .await
                {
                    logging::log_warning(&format!(
                        "[{}] ⚠️ Failed to mark spent charms for reindex block {}: {}",
                        network_id.name, height, e
                    ));
                }
            }
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Could not fetch block {} for spent tracking (pruned?): {}",
                    network_id.name, height, e
                ));
            }
        },
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Could not get block hash for height {} during reindex: {}",
                network_id.name, height, e
            ));
        }
    }

    let _ = block_status_repository
        .mark_processed(height as i32, charm_count, network_id)
        .await;

    logging::log_info(&format!(
        "[{}] ♻️ Reindex Block {}: Tx {} | Charms {}",
        network_id.name,
        height,
        cached_txs.len(),
        charm_count,
    ));

    Ok(())
}
