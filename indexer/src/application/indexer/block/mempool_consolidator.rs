//! Promotes mempool entries to confirmed status when their block arrives.
//! Idempotent — safe to call even if no mempool entries exist.

use bitcoincore_rpc::bitcoin;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use crate::config::NetworkId;
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// Consolidate mempool entries for txs in this block:
/// 1. Update charms.block_height to confirmed height
/// 2. Update transactions status to 'confirmed'
/// 3. Update dex_orders.block_height
/// 4. Remove mempool_spends entries
pub async fn consolidate(
    block: &bitcoin::Block,
    height: u64,
    network_id: &NetworkId,
    mempool_spends_repository: &MempoolSpendsRepository,
) {
    let network = &network_id.name;

    let txids: Vec<String> = block
        .txdata
        .iter()
        .map(|tx| tx.txid().to_string())
        .collect();

    if txids.is_empty() {
        return;
    }

    let id_list: Vec<String> = txids
        .iter()
        .map(|id| format!("'{}'", id.replace('\'', "''")))
        .collect();
    let ids_sql = id_list.join(", ");
    let conn = mempool_spends_repository.get_connection();

    // 1. Promote mempool charms to confirmed block_height
    let sql = format!(
        "UPDATE charms SET block_height = {}, mempool_detected_at = mempool_detected_at \
         WHERE txid IN ({}) AND network = '{}' AND block_height IS NULL",
        height, ids_sql, network
    );
    match conn.execute(Statement::from_string(DbBackend::Postgres, sql)).await {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] ✅ Block {}: Promoted {} mempool charms to confirmed",
                network, height, r.rows_affected()
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Block {}: Failed to promote mempool charms: {}",
                network, height, e
            ));
        }
    }

    // 2. Promote mempool transactions to confirmed
    let sql = format!(
        "UPDATE transactions SET block_height = {}, status = 'confirmed', updated_at = NOW() \
         WHERE txid IN ({}) AND network = '{}' AND (block_height IS NULL OR status = 'pending')",
        height, ids_sql, network
    );
    match conn.execute(Statement::from_string(DbBackend::Postgres, sql)).await {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] ✅ Block {}: Promoted {} mempool transactions to confirmed",
                network, height, r.rows_affected()
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Block {}: Failed to promote mempool transactions: {}",
                network, height, e
            ));
        }
    }

    // 3. Promote mempool DEX orders to confirmed
    let sql = format!(
        "UPDATE dex_orders SET block_height = {}, updated_at = NOW() \
         WHERE txid IN ({}) AND network = '{}' AND block_height IS NULL",
        height, ids_sql, network
    );
    if let Err(e) = conn.execute(Statement::from_string(DbBackend::Postgres, sql)).await {
        logging::log_warning(&format!(
            "[{}] ⚠️ Block {}: Failed to promote mempool DEX orders: {}",
            network, height, e
        ));
    }

    // 4. Remove mempool_spends for confirmed txs
    if let Err(e) = mempool_spends_repository
        .remove_confirmed_spends(&txids, network)
        .await
    {
        logging::log_warning(&format!(
            "[{}] ⚠️ Block {}: Failed to remove confirmed mempool_spends: {}",
            network, height, e
        ));
    }
}
