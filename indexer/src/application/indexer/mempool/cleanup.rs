//! Stale mempool entry purging: removes charms/orders/spends older than 24h.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::collections::HashSet;
use tokio::sync::Mutex;

use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// How many hours before a mempool entry is considered stale
const STALE_HOURS: i64 = 24;

/// Purge stale mempool entries and trim the seen_txids cache.
pub async fn purge_stale(
    network: &str,
    db: &DatabaseConnection,
    mempool_spends_repository: &MempoolSpendsRepository,
    seen_txids: &std::sync::Arc<Mutex<HashSet<String>>>,
) {
    // 1. Purge stale mempool_spends
    match mempool_spends_repository.purge_stale(STALE_HOURS).await {
        Ok(n) if n > 0 => {
            logging::log_info(&format!(
                "[{}] 🧹 Purged {} stale mempool_spends entries",
                network, n
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to purge stale mempool_spends: {}",
                network, e
            ));
        }
    }

    // 2. Get stale charm info BEFORE deletion (for stats_holders reversal) then purge
    let get_stale_sql = format!(
        "SELECT app_id, address, amount FROM charms WHERE block_height IS NULL AND network = '{}' \
         AND mempool_detected_at < NOW() - INTERVAL '{} hours' AND address IS NOT NULL",
        network, STALE_HOURS
    );
    let stale_charms = db
        .query_all(Statement::from_string(DbBackend::Postgres, get_stale_sql))
        .await
        .unwrap_or_default();

    let sql = format!(
        "DELETE FROM charms WHERE block_height IS NULL AND network = '{}' \
         AND mempool_detected_at < NOW() - INTERVAL '{} hours'",
        network, STALE_HOURS
    );
    match db
        .execute(Statement::from_string(DbBackend::Postgres, sql))
        .await
    {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] 🧹 Purged {} stale mempool charms",
                network,
                r.rows_affected()
            ));

            // Revert stats_holders for purged charms
            for row in &stale_charms {
                if let (Ok(app_id), Ok(addr), Ok(amount)) = (
                    row.try_get::<String>("", "app_id"),
                    row.try_get::<String>("", "address"),
                    row.try_get::<i64>("", "amount"),
                ) {
                    let (holder_app_id, delta) = if app_id.starts_with("t/") {
                        (app_id.replacen("t/", "n/", 1), -amount)
                    } else if app_id.starts_with("n/") {
                        (app_id.clone(), -1_i64)
                    } else {
                        continue;
                    };
                    let update_sql = format!(
                        r#"UPDATE stats_holders SET total_amount = total_amount + {}, charm_count = charm_count - 1, updated_at = CURRENT_TIMESTAMP
                           WHERE app_id = '{}' AND address = '{}'"#,
                        delta,
                        holder_app_id.replace('\'', "''"),
                        addr.replace('\'', "''")
                    );
                    let _ = db.execute(Statement::from_string(DbBackend::Postgres, update_sql)).await;
                    let cleanup_sql = format!(
                        "DELETE FROM stats_holders WHERE app_id = '{}' AND address = '{}' AND total_amount <= 0",
                        holder_app_id.replace('\'', "''"),
                        addr.replace('\'', "''")
                    );
                    let _ = db.execute(Statement::from_string(DbBackend::Postgres, cleanup_sql)).await;
                }
            }
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to purge stale mempool charms: {}",
                network, e
            ));
        }
    }

    // 3. Purge stale mempool DEX orders
    let sql_orders = format!(
        "DELETE FROM dex_orders WHERE block_height IS NULL AND network = '{}' \
         AND created_at < NOW() - INTERVAL '{} hours'",
        network, STALE_HOURS
    );
    let _ = db
        .execute(Statement::from_string(DbBackend::Postgres, sql_orders))
        .await;

    // 4. Purge stale mempool transactions
    let sql_txs = format!(
        "DELETE FROM transactions WHERE block_height IS NULL AND network = '{}' \
         AND mempool_detected_at < NOW() - INTERVAL '{} hours'",
        network, STALE_HOURS
    );
    match db
        .execute(Statement::from_string(DbBackend::Postgres, sql_txs))
        .await
    {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] 🧹 Purged {} stale mempool transactions",
                network,
                r.rows_affected()
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to purge stale mempool transactions: {}",
                network, e
            ));
        }
    }

    // seen_txids is kept in sync with the live mempool via retain() in poll_once —
    // no explicit clearing needed here.
    let _ = seen_txids; // suppress unused warning
}
