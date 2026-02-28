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

    // 2. Purge stale mempool charms
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

    // 4. Trim seen_txids cache to prevent unbounded growth
    let mut seen = seen_txids.lock().await;
    if seen.len() > 10_000 {
        seen.clear();
        logging::log_info(&format!(
            "[{}] 🧹 Cleared seen_txids cache (was >10k entries)",
            network
        ));
    }
}
