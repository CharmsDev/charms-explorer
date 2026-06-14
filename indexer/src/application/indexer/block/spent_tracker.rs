//! Marks charms as spent by analyzing transaction inputs in a block.

use bitcoincore_rpc::bitcoin;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;

use super::retry::RetryHandler;

/// Collect all (txid, vout) pairs being spent in a block, mark them as
/// spent in `charms`, and return the negative holder deltas so the block
/// processor can merge them with the additive deltas before calling
/// `update_holders_batch` once per (app_id, address) per block.
pub async fn mark_spent_charms(
    block: &bitcoin::Block,
    height: u64,
    network_id: &NetworkId,
    charm_service: &CharmService,
    retry_handler: &RetryHandler,
) -> Result<Vec<(String, String, i64, i32)>, BlockProcessorError> {
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

    if spent_txid_vouts.is_empty() {
        return Ok(Vec::new());
    }

    let block_height = height as i32;
    let deltas = retry_handler
        .execute_with_retry_and_logging(
            || async {
                charm_service
                    .mark_charms_as_spent_batch(
                        spent_txid_vouts.clone(),
                        &network_id.name,
                        block_height,
                    )
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

    Ok(deltas)
}
