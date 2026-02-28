//! Marks charms as spent by analyzing transaction inputs in a block.

use bitcoincore_rpc::bitcoin;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;

use super::retry::RetryHandler;

/// Collect all (txid, vout) pairs being spent in a block and mark them as spent.
pub async fn mark_spent_charms(
    block: &bitcoin::Block,
    network_id: &NetworkId,
    charm_service: &CharmService,
    retry_handler: &RetryHandler,
) -> Result<(), BlockProcessorError> {
    let mut spent_txid_vouts: Vec<(String, i32)> = Vec::new();

    for tx in &block.txdata {
        if tx.is_coin_base() {
            continue;
        }
        for input in &tx.input {
            spent_txid_vouts.push((
                input.previous_output.txid.to_string(),
                input.previous_output.vout as i32,
            ));
        }
    }

    if !spent_txid_vouts.is_empty() {
        retry_handler
            .execute_with_retry_and_logging(
                || async {
                    charm_service
                        .mark_charms_as_spent_batch(spent_txid_vouts.clone())
                        .await
                        .map_err(|e| {
                            crate::infrastructure::persistence::error::DbError::QueryError(
                                e.to_string(),
                            )
                        })
                },
                "mark charms as spent",
                &network_id.name,
            )
            .await
            .map_err(BlockProcessorError::DbError)?;
    }

    Ok(())
}
