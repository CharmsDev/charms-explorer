//! Reorg detection and recovery for the Bitcoin block processor.
//!
//! Strategy
//! --------
//! Before processing block `N`, compare `block.header.prev_blockhash` to the
//! `block_hash` stored for height `N-1`. If they differ, walk backwards
//! comparing on-chain hashes to stored hashes until we find the common
//! ancestor; everything above is wiped and the indexer resumes from there.
//!
//! Tables wiped on rollback (idempotent — all use `DELETE WHERE block_height > h`):
//! - `charms`, `transactions`, `assets`, `address_utxos`, `block_status`
//! - `dex_orders` are marked `status='reorged'` instead of deleted (audit trail).
//! - `mempool_spends` are fully cleared (mempool re-emerges naturally).
//! - `stats_holders` is invalidated by deleting rows above the divergence;
//!   subsequent block processing repopulates them via UPSERT.

use sea_orm::{ConnectionTrait, DbBackend, Statement};

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    BlockStatusRepository, ReorgEventsRepository,
};
use crate::utils::metrics;

/// Outcome of the pre-processing reorg check.
pub enum ReorgDecision {
    /// Safe to process this block normally.
    None,
    /// Reorg detected; the indexer rolled back to `divergence_height` and the
    /// caller should restart from `divergence_height + 1`.
    RolledBackTo(u64),
}

/// Check the chain against `block.header.prev_blockhash` and roll back if a
/// reorg is detected. Idempotent: returns `None` for height = genesis or when
/// there is no prior entry in `block_status`.
pub async fn check_and_recover(
    height: u64,
    block: &bitcoincore_rpc::bitcoin::Block,
    network_id: &NetworkId,
    bitcoin_client: &BitcoinClient,
    block_status: &BlockStatusRepository,
    reorg_events: &ReorgEventsRepository,
) -> Result<ReorgDecision, BlockProcessorError> {
    if height == 0 {
        return Ok(ReorgDecision::None);
    }

    let prev_height = (height - 1) as i32;
    let stored_prev = block_status
        .get_block_hash(prev_height, network_id)
        .await
        .map_err(|e| BlockProcessorError::ProcessingError(format!("block_status read: {}", e)))?;

    let Some(stored_prev) = stored_prev else {
        // No prior record: either fresh start at genesis or DB was wiped above.
        return Ok(ReorgDecision::None);
    };

    let on_chain_prev = block.header.prev_blockhash.to_string();
    if on_chain_prev == stored_prev {
        return Ok(ReorgDecision::None);
    }

    tracing::warn!(
        height,
        network = %network_id.name,
        stored = %stored_prev,
        on_chain = %on_chain_prev,
        "reorg detected at block boundary"
    );

    let divergence = find_divergence_point(prev_height, network_id, bitcoin_client, block_status)
        .await?;
    let depth = (height as i32) - divergence;
    let event_id = reorg_events
        .record(network_id, height as i32, depth)
        .await
        .map_err(|e| BlockProcessorError::ProcessingError(format!("reorg_events: {}", e)))?;

    rollback_above(divergence, network_id, block_status).await?;

    let _ = reorg_events.mark_recovered(event_id).await;
    metrics::reorg_detected(&network_id.name, depth as u64);

    Ok(ReorgDecision::RolledBackTo(divergence as u64))
}

/// Walk backwards comparing stored hash to on-chain hash; return the first
/// height where they agree (the common ancestor). Falls back to genesis-1 if
/// no agreement is found within `MAX_DEPTH` blocks.
async fn find_divergence_point(
    mut height: i32,
    network_id: &NetworkId,
    bitcoin_client: &BitcoinClient,
    block_status: &BlockStatusRepository,
) -> Result<i32, BlockProcessorError> {
    const MAX_DEPTH: i32 = 100;
    let mut steps = 0;

    while height >= 0 && steps < MAX_DEPTH {
        let stored = block_status
            .get_block_hash(height, network_id)
            .await
            .map_err(|e| BlockProcessorError::ProcessingError(format!("hash read: {}", e)))?;
        let Some(stored) = stored else {
            return Ok(height);
        };
        let on_chain = bitcoin_client
            .get_block_hash(height as u64)
            .await
            .map_err(BlockProcessorError::BitcoinClientError)?
            .to_string();
        if stored == on_chain {
            return Ok(height);
        }
        height -= 1;
        steps += 1;
    }

    tracing::error!(
        height,
        network = %network_id.name,
        "reorg walk exceeded MAX_DEPTH; rolling back to lowest checked height"
    );
    Ok(height.max(-1))
}

/// Wipe indexer state above `height` (exclusive).
async fn rollback_above(
    height: i32,
    network_id: &NetworkId,
    block_status: &BlockStatusRepository,
) -> Result<(), BlockProcessorError> {
    let conn = block_status.get_connection();
    let net = network_id.name.as_str();

    let statements: &[&str] = &[
        "DELETE FROM charms WHERE block_height > $1 AND network = $2",
        "DELETE FROM transactions WHERE block_height > $1 AND network = $2",
        "DELETE FROM assets WHERE block_height > $1 AND network = $2",
        "DELETE FROM address_utxos WHERE block_height > $1 AND network = $2",
        "UPDATE dex_orders SET status = 'reorged' WHERE block_height > $1 AND network = $2",
        "DELETE FROM mempool_spends WHERE network = $1",
        "DELETE FROM stats_holders WHERE last_updated_block > $1 AND network = $2",
    ];

    for (i, sql) in statements.iter().enumerate() {
        let values: Vec<sea_orm::Value> = if i == 5 {
            vec![net.into()]
        } else {
            vec![height.into(), net.into()]
        };
        conn.execute(Statement::from_sql_and_values(DbBackend::Postgres, *sql, values))
            .await
            .map_err(|e| {
                BlockProcessorError::ProcessingError(format!("rollback step {}: {}", i, e))
            })?;
    }

    let removed = block_status
        .delete_above(height, network_id)
        .await
        .map_err(|e| BlockProcessorError::ProcessingError(format!("block_status delete: {}", e)))?;

    tracing::info!(
        divergence = height,
        block_status_removed = removed,
        network = %network_id.name,
        "reorg rollback completed"
    );
    Ok(())
}
