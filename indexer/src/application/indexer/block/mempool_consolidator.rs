//! Promotes mempool entries to confirmed status when their block arrives —
//! but only the ones whose ZK proof passed strict verification in the
//! block path (Plan 15). Mempool rows for txs that confirmed without
//! verifying are PURGED so confirmed tables never carry unverified state.
//! Idempotent — safe to call even if no mempool entries exist.

use std::collections::HashSet;

use bitcoincore_rpc::bitcoin;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use crate::config::NetworkId;
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// Consolidate mempool entries for txs in this block:
/// 1. Purge mempool rows for txs in the block that did NOT verify in the
///    block path (their spell either failed ZK verification or was a
///    permissive-only artifact).
/// 2. Promote the remaining (verified) mempool rows to the confirmed height.
/// 3. Remove mempool_spends entries for every confirmed tx.
pub async fn consolidate(
    block: &bitcoin::Block,
    height: u64,
    network_id: &NetworkId,
    mempool_spends_repository: &MempoolSpendsRepository,
    verified_txids: &HashSet<String>,
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

    // Split block txids into two groups:
    //   - `verified`: passed Strict ZK in detection. Their mempool rows
    //     (if any) get promoted to confirmed.
    //   - `unverified`: in this block but absent from detection (regular
    //     non-charm txs OR permissive-only spell-shapes that failed the
    //     proof). Any mempool row sitting on them is a false positive
    //     that must be purged.
    let (verified, unverified): (Vec<String>, Vec<String>) = txids
        .iter()
        .cloned()
        .partition(|t| verified_txids.contains(t));

    let conn = mempool_spends_repository.get_connection();

    // 0. PURGE unverified false positives. Charm / tx / dex_order /
    //    address_utxo mempool rows linked to these txids are removed.
    //    mempool_spends keyed by spending_txid are also removed.
    if !unverified.is_empty() {
        let uv_sql = unverified
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");

        for table_and_pred in [
            ("charms", "block_height IS NULL"),
            ("transactions", "block_height IS NULL"),
            ("dex_orders", "block_height IS NULL"),
            ("address_utxos", "block_height = 0"),
        ] {
            let (table, pred) = table_and_pred;
            let sql = format!(
                "DELETE FROM {} WHERE txid IN ({}) AND network = '{}' AND {}",
                table, uv_sql, network, pred
            );
            if let Err(e) = conn
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await
            {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Block {}: purge {} for unverified txs failed: {}",
                    network, height, table, e
                ));
            }
        }
        let sql = format!(
            "DELETE FROM mempool_spends WHERE spending_txid IN ({}) AND network = '{}'",
            uv_sql, network
        );
        if let Err(e) = conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
        {
            logging::log_warning(&format!(
                "[{}] ⚠️ Block {}: purge mempool_spends for unverified txs failed: {}",
                network, height, e
            ));
        }
        logging::log_info(&format!(
            "[{}] 🧹 Block {}: purged mempool rows for {} unverified tx(s)",
            network, height, unverified.len()
        ));
    }

    if verified.is_empty() {
        // No verified mempool rows to promote; mempool_spends still needs
        // cleanup for these block txids in case there were spend records.
        if let Err(e) = mempool_spends_repository
            .remove_confirmed_spends(&txids, network)
            .await
        {
            logging::log_warning(&format!(
                "[{}] ⚠️ Block {}: Failed to remove confirmed mempool_spends: {}",
                network, height, e
            ));
        }
        return;
    }

    let ids_sql = verified
        .iter()
        .map(|id| format!("'{}'", id.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

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

    // 5. Promote unconfirmed address_utxos to confirmed block_height
    let sql = format!(
        "UPDATE address_utxos SET block_height = {} \
         WHERE txid IN ({}) AND network = '{}' AND block_height = 0",
        height, ids_sql, network
    );
    match conn.execute(Statement::from_string(DbBackend::Postgres, sql)).await {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] ✅ Block {}: Promoted {} mempool address_utxos to confirmed",
                network, height, r.rows_affected()
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Block {}: Failed to promote mempool address_utxos: {}",
                network, height, e
            ));
        }
    }
}
