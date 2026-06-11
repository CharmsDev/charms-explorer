//! Stale mempool entry purging: removes charms/orders/spends older than 24h.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
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

    // 2. Purge stale mempool charms
    // stats_holders is not affected — mempool charms never update stats_holders.
    exec_or_warn(
        db,
        network,
        "stale mempool charms",
        &format!(
            "DELETE FROM charms WHERE block_height IS NULL AND network = $1 \
             AND mempool_detected_at < NOW() - INTERVAL '{} hours'",
            STALE_HOURS
        ),
        vec![network.into()],
    )
    .await;

    // 3. Purge stale mempool DEX orders
    exec_or_warn(
        db,
        network,
        "stale mempool DEX orders",
        &format!(
            "DELETE FROM dex_orders WHERE block_height IS NULL AND network = $1 \
             AND created_at < NOW() - INTERVAL '{} hours'",
            STALE_HOURS
        ),
        vec![network.into()],
    )
    .await;

    // 4. Purge stale mempool transactions
    exec_or_warn(
        db,
        network,
        "stale mempool transactions",
        &format!(
            "DELETE FROM transactions WHERE block_height IS NULL AND network = $1 \
             AND mempool_detected_at < NOW() - INTERVAL '{} hours'",
            STALE_HOURS
        ),
        vec![network.into()],
    )
    .await;

    // 5. Purge orphaned address_utxos (block_height=0 with no matching pending tx)
    exec_or_warn(
        db,
        network,
        "orphaned mempool address_utxos",
        "DELETE FROM address_utxos WHERE block_height = 0 AND network = $1 \
         AND txid NOT IN (SELECT txid FROM transactions WHERE block_height IS NULL AND network = $2)",
        vec![network.into(), network.into()],
    )
    .await;

    // seen_txids is kept in sync with the live mempool via retain() in poll_once —
    // no explicit clearing needed here.
    let _ = seen_txids; // suppress unused warning
}

/// Execute a DELETE with bind parameters and log success/failure consistently.
/// Parameterising `network` closes the defense-in-depth gap that the audit
/// flagged as N9 — even though the value comes from config today, this
/// removes the SQL-interpolation footgun for the next network we add.
async fn exec_or_warn(
    db: &DatabaseConnection,
    network: &str,
    label: &str,
    sql: &str,
    params: Vec<Value>,
) {
    let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, params);
    match db.execute(stmt).await {
        Ok(r) if r.rows_affected() > 0 => {
            logging::log_info(&format!(
                "[{}] 🧹 Purged {} {}",
                network,
                r.rows_affected(),
                label
            ));
        }
        Ok(_) => {}
        Err(e) => {
            logging::log_warning(&format!("[{}] ⚠️ Failed to purge {}: {}", network, label, e));
        }
    }
}
