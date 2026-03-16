//! Mempool processing module — polls Bitcoin Core's mempool for charm transactions.
//!
//! Sub-modules:
//! - `processor`: core detection + persistence for individual mempool txs
//! - `cleanup`: stale entry purging

mod cleanup;
mod processor;
mod reconcile;
pub mod utxo_tracker;

use std::collections::HashSet;
use std::time::Duration;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

use crate::config::NetworkId;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    MempoolSpendsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::utils::logging;

/// How often to poll the mempool (seconds)
const POLL_INTERVAL_SECS: u64 = 1;

/// Maximum number of mempool txids to process per poll cycle
const MAX_TXS_PER_CYCLE: usize = 100;

/// How often to reload the monitored address set (every N cycles)
const MONITORED_SET_RELOAD_INTERVAL: u64 = 60;

/// How often to reconcile DB state with the live mempool (every N cycles).
/// At 1s per cycle, 300 cycles = every 5 minutes.
const RECONCILE_INTERVAL_CYCLES: u64 = 300;

/// Mempool processor — runs as a background task alongside the block processor
pub struct MempoolProcessor {
    bitcoin_client: BitcoinClient,
    db: DatabaseConnection,
    mempool_spends_repository: MempoolSpendsRepository,
    utxo_repository: UtxoRepository,
    monitored_addresses_repository: MonitoredAddressesRepository,
    network_id: NetworkId,
    seen_txids: std::sync::Arc<Mutex<HashSet<String>>>,
    monitored_set: std::sync::Arc<Mutex<HashSet<String>>>,
}

impl MempoolProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        db: DatabaseConnection,
        mempool_spends_repository: MempoolSpendsRepository,
        utxo_repository: UtxoRepository,
        monitored_addresses_repository: MonitoredAddressesRepository,
        network_id: NetworkId,
    ) -> Self {
        Self {
            bitcoin_client,
            db,
            mempool_spends_repository,
            utxo_repository,
            monitored_addresses_repository,
            network_id,
            seen_txids: std::sync::Arc::new(Mutex::new(HashSet::new())),
            monitored_set: std::sync::Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Main loop — runs forever, polling mempool every POLL_INTERVAL_SECS
    pub async fn run(&self) {
        logging::log_info(&format!(
            "[{}] 🔍 MempoolProcessor started (poll every {}s)",
            self.network_id.name, POLL_INTERVAL_SECS
        ));

        let mut cycle: u64 = 0;
        // Load monitored set on startup
        self.reload_monitored_set().await;

        loop {
            cycle += 1;

            // Reload monitored set periodically
            if cycle % MONITORED_SET_RELOAD_INTERVAL == 0 {
                self.reload_monitored_set().await;
            }

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

            // Reconcile: revert side effects for txs that left the mempool
            if cycle % RECONCILE_INTERVAL_CYCLES == 0 {
                self.reconcile_with_mempool().await;
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

        // Build a set of current mempool txids for O(1) lookup
        let mempool_set: HashSet<&str> = mempool_txids.iter().map(String::as_str).collect();

        // Diff against seen set.
        // IMPORTANT: retain() removes txids that left the mempool (confirmed or dropped).
        // This keeps seen_txids naturally bounded to mempool size and ensures truly new
        // txids are always detected on their first poll cycle — no bulk re-scanning.
        let new_txids: Vec<String> = {
            let mut seen = self.seen_txids.lock().await;
            seen.retain(|txid| mempool_set.contains(txid.as_str()));
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
            self.network_id.name,
            cycle,
            new_txids.len()
        ));

        let mut charm_count = 0usize;
        let mut order_count = 0usize;

        // Get a snapshot of the monitored set for this cycle
        let monitored_snapshot = self.monitored_set.lock().await.clone();

        for txid in &new_txids {
            // Track UTXOs for monitored addresses (ALL txs, not just charm txs)
            // We need the raw hex for both charm detection and UTXO tracking
            let raw_hex = match self
                .bitcoin_client
                .get_raw_transaction_hex(txid, None)
                .await
            {
                Ok(hex) => hex,
                Err(e) => {
                    logging::log_debug(&format!(
                        "[{}] Mempool tx {} hex fetch failed: {}",
                        self.network_id.name, txid, e
                    ));
                    continue;
                }
            };

            // Track UTXO changes for monitored addresses
            if !monitored_snapshot.is_empty() {
                utxo_tracker::track_mempool_utxos(
                    txid,
                    &raw_hex,
                    &self.network_id.name,
                    &monitored_snapshot,
                    &self.utxo_repository,
                    &self.mempool_spends_repository,
                )
                .await;
            }

            // Detect charms (pass raw_hex to avoid re-fetching)
            match processor::process_tx_with_hex(
                txid,
                &raw_hex,
                &self.network_id,
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
                        "[{}] Mempool tx {} charm detection skipped: {}",
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

    /// Reconcile DB with live mempool: revert all side effects for dropped txs.
    async fn reconcile_with_mempool(&self) {
        let mempool_txids = match self.bitcoin_client.get_raw_mempool().await {
            Ok(txids) => txids,
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Reconcile: getrawmempool failed: {}",
                    self.network_id.name, e
                ));
                return;
            }
        };

        let live_set: std::collections::HashSet<String> =
            mempool_txids.into_iter().collect();

        let reverted = reconcile::reconcile_dropped_txs(
            &self.network_id.name,
            &live_set,
            &self.db,
            &self.mempool_spends_repository,
        )
        .await;

        if reverted > 0 {
            // Also remove reverted txids from seen_txids so they don't block re-detection
            // if the same tx re-enters the mempool later
            let mut seen = self.seen_txids.lock().await;
            seen.retain(|txid| live_set.contains(txid));
        }
    }

    /// Reload the monitored address set from DB
    async fn reload_monitored_set(&self) {
        let set = utxo_tracker::load_monitored_set(
            &self.network_id.name,
            &self.monitored_addresses_repository,
        )
        .await;
        let count = set.len();
        *self.monitored_set.lock().await = set;
        logging::log_info(&format!(
            "[{}] 📡 Mempool UTXO tracker: {} seeded addresses loaded",
            self.network_id.name, count
        ));
    }
}
