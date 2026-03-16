//! Mempool reconciliation: detects transactions that have disappeared from the
//! Bitcoin Core mempool (dropped, RBF-replaced, or evicted) and reverts all
//! side effects that were recorded when they were first detected.
//!
//! Side effects reverted per dropped tx (in order):
//!   1. dex_orders  — revert parent order status back to "open" (FULFILL/CANCEL)
//!   2. dex_orders  — delete activity rows AND create-order rows for this txid
//!   3. charms      — delete charm entries (block_height IS NULL)
//!   4. transactions — delete transaction entry (block_height IS NULL)
//!   5. mempool_spends — delete spend records by spending_txid
//!   6. address_utxos  — delete unconfirmed UTXOs (block_height = 0)

use std::collections::HashSet;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// Reconcile DB state with the live mempool.
///
/// Fetches all pending (block_height IS NULL) transaction txids from the DB,
/// compares against the current mempool set from Bitcoin Core, and reverts all
/// side effects for transactions that are no longer present.
///
/// Returns the number of transactions successfully reverted.
pub async fn reconcile_dropped_txs(
    network: &str,
    live_mempool: &HashSet<String>,
    db: &DatabaseConnection,
    mempool_spends_repository: &MempoolSpendsRepository,
) -> usize {
    // 1. Get all pending txids from transactions table
    let pending_txids = match get_pending_txids(network, db).await {
        Ok(txids) => txids,
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Reconcile: failed to fetch pending txids: {}",
                network, e
            ));
            return 0;
        }
    };

    if pending_txids.is_empty() {
        return 0;
    }

    // 2. Find txids that are NOT in the live mempool (dropped/evicted/replaced)
    let dropped: Vec<&String> = pending_txids
        .iter()
        .filter(|txid| !live_mempool.contains(txid.as_str()))
        .collect();

    if dropped.is_empty() {
        logging::log_debug(&format!(
            "[{}] Reconcile: all {} pending txs still in mempool",
            network,
            pending_txids.len()
        ));
        return 0;
    }

    logging::log_info(&format!(
        "[{}] 🔄 Reconcile: {} of {} pending txs dropped from mempool — reverting",
        network,
        dropped.len(),
        pending_txids.len()
    ));

    let mut reverted = 0usize;

    for txid in &dropped {
        match revert_mempool_tx(txid, network, db, mempool_spends_repository).await {
            Ok(()) => reverted += 1,
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Reconcile: failed to revert tx {}: {}",
                    network, txid, e
                ));
            }
        }
    }

    if reverted > 0 {
        logging::log_info(&format!(
            "[{}] ✅ Reconcile: {} transactions fully reverted",
            network, reverted
        ));
    }

    reverted
}

/// Get all txids with block_height IS NULL (mempool/pending) from the transactions table.
async fn get_pending_txids(
    network: &str,
    db: &DatabaseConnection,
) -> Result<Vec<String>, String> {
    let sql = format!(
        "SELECT txid FROM transactions WHERE block_height IS NULL AND network = '{}'",
        network.replace('\'', "''")
    );

    let rows = db
        .query_all(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .map_err(|e| e.to_string())?;

    let txids: Vec<String> = rows
        .iter()
        .filter_map(|row| row.try_get::<String>("", "txid").ok())
        .collect();

    Ok(txids)
}

/// Revert ALL side effects of a single mempool transaction that has been dropped.
///
/// Order matters: parent order status must be restored before deleting the
/// activity row, and mempool_spends must be cleaned before address_utxos
/// so that wallet balance queries never see an inconsistent state.
async fn revert_mempool_tx(
    txid: &str,
    network: &str,
    db: &DatabaseConnection,
    mempool_spends_repository: &MempoolSpendsRepository,
) -> Result<(), String> {
    let escaped_txid = txid.replace('\'', "''");
    let escaped_network = network.replace('\'', "''");

    // 1. Revert parent order status for FULFILL/CANCEL operations.
    //    Activity rows have parent_order_id set — restore the parent back to "open".
    let revert_parent_sql = format!(
        "UPDATE dex_orders SET status = 'open', updated_at = NOW() \
         WHERE order_id IN (\
             SELECT parent_order_id FROM dex_orders \
             WHERE txid = '{}' AND network = '{}' AND parent_order_id IS NOT NULL\
         )",
        escaped_txid, escaped_network
    );
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        revert_parent_sql,
    ))
    .await
    .map_err(|e| format!("revert parent order: {}", e))?;

    // 2. Delete dex_orders rows for this txid (CREATE orders + activity rows)
    let del_orders_sql = format!(
        "DELETE FROM dex_orders WHERE txid = '{}' AND network = '{}' AND block_height IS NULL",
        escaped_txid, escaped_network
    );
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        del_orders_sql,
    ))
    .await
    .map_err(|e| format!("delete dex_orders: {}", e))?;

    // 3. Delete charms entries
    let del_charms_sql = format!(
        "DELETE FROM charms WHERE txid = '{}' AND network = '{}' AND block_height IS NULL",
        escaped_txid, escaped_network
    );
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        del_charms_sql,
    ))
    .await
    .map_err(|e| format!("delete charms: {}", e))?;

    // 4. Delete transactions entry
    let del_tx_sql = format!(
        "DELETE FROM transactions WHERE txid = '{}' AND network = '{}' AND block_height IS NULL",
        escaped_txid, escaped_network
    );
    db.execute(Statement::from_string(DbBackend::Postgres, del_tx_sql))
        .await
        .map_err(|e| format!("delete transactions: {}", e))?;

    // 5. Delete mempool_spends (all inputs this tx was consuming)
    mempool_spends_repository
        .remove_by_spending_txid(txid, network)
        .await
        .map_err(|e| format!("delete mempool_spends: {}", e))?;

    // 6. Delete unconfirmed address_utxos created by this tx (block_height = 0)
    let del_utxos_sql = format!(
        "DELETE FROM address_utxos WHERE txid = '{}' AND network = '{}' AND block_height = 0",
        escaped_txid, escaped_network
    );
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        del_utxos_sql,
    ))
    .await
    .map_err(|e| format!("delete address_utxos: {}", e))?;

    logging::log_info(&format!(
        "[{}] 🗑️ Reverted dropped mempool tx {}",
        network, txid
    ));

    Ok(())
}
