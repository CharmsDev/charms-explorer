//! Mempool processing module — polls Bitcoin Core's mempool for charm transactions.
//!
//! Sub-modules:
//! - `processor`: core detection + persistence for individual mempool txs
//! - `cleanup`: stale entry purging

mod cleanup;
mod processor;

use std::collections::HashSet;
use std::time::Duration;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

use crate::config::NetworkId;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// How often to poll the mempool (seconds)
const POLL_INTERVAL_SECS: u64 = 1;

/// Maximum number of mempool txids to process per poll cycle
const MAX_TXS_PER_CYCLE: usize = 100;

/// Mempool processor — runs as a background task alongside the block processor
pub struct MempoolProcessor {
    bitcoin_client: BitcoinClient,
    db: DatabaseConnection,
    mempool_spends_repository: MempoolSpendsRepository,
    network_id: NetworkId,
    seen_txids: std::sync::Arc<Mutex<HashSet<String>>>,
}

impl MempoolProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        db: DatabaseConnection,
        mempool_spends_repository: MempoolSpendsRepository,
        network_id: NetworkId,
    ) -> Self {
        Self {
            bitcoin_client,
            db,
            mempool_spends_repository,
            network_id,
            seen_txids: std::sync::Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Main loop — runs forever, polling mempool every POLL_INTERVAL_SECS
    pub async fn run(&self) {
        logging::log_info(&format!(
            "[{}] 🔍 MempoolProcessor started (poll every {}s)",
            self.network_id.name, POLL_INTERVAL_SECS
        ));

        let mut cycle: u64 = 0;
        loop {
            cycle += 1;

            if let Err(e) = self.poll_once(cycle).await {
                logging::log_warning(&format!(
                    "[{}] ⚠️ MempoolProcessor cycle {} error: {}",
                    self.network_id.name, cycle, e
                ));
            }

            // Purge stale entries every 100 cycles
            if cycle % 100 == 0 {
                cleanup::purge_stale(
                    &self.network_id.name,
                    &self.db,
                    &self.mempool_spends_repository,
                    &self.seen_txids,
                )
                .await;
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    }

    /// Single poll cycle: fetch mempool, detect new charm txs, save them
    async fn poll_once(&self, cycle: u64) -> Result<(), String> {
        let mempool_txids = self
            .bitcoin_client
            .get_raw_mempool()
            .await
            .map_err(|e| format!("getrawmempool failed: {}", e))?;

        if mempool_txids.is_empty() {
            return Ok(());
        }

        // Diff against seen set
        let new_txids: Vec<String> = {
            let mut seen = self.seen_txids.lock().await;
            let new: Vec<String> = mempool_txids
                .into_iter()
                .filter(|txid| !seen.contains(txid))
                .take(MAX_TXS_PER_CYCLE)
                .collect();
            for txid in &new {
                seen.insert(txid.clone());
            }
            new
        };

        if new_txids.is_empty() {
            return Ok(());
        }

        logging::log_info(&format!(
            "[{}] 🔍 Mempool cycle {}: {} new txids to check",
            self.network_id.name, cycle, new_txids.len()
        ));

        let mut charm_count = 0usize;
        let mut order_count = 0usize;

        for txid in &new_txids {
            match processor::process_tx(
                txid,
                &self.network_id,
                &self.bitcoin_client,
                &self.db,
                &self.mempool_spends_repository,
            )
            .await
            {
                Ok(Some(detected)) => {
                    charm_count += 1;
                    if detected.has_dex_order {
                        order_count += 1;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    logging::log_debug(&format!(
                        "[{}] Mempool tx {} skipped: {}",
                        self.network_id.name, txid, e
                    ));
                }
            }
        }

        if charm_count > 0 {
            logging::log_info(&format!(
                "[{}] ✅ Mempool cycle {}: {} charms detected ({} DEX orders)",
                self.network_id.name, cycle, charm_count, order_count
            ));
        }

        Ok(())
    }
}
