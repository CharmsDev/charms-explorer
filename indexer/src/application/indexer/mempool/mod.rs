//! Mempool processing module — consumes the mempool.space websocket feed
//! and detects charm transactions in it.
//!
//! Sub-modules:
//! - `processor`: core detection + persistence for individual mempool txs
//! - `cleanup`: stale entry purging

mod cleanup;
mod dex_persistence;
mod processor;
mod reconcile;
mod spend_extraction;
pub mod utxo_tracker;

use std::collections::HashSet;
use std::time::Duration;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

use crate::config::NetworkId;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::bitcoin::MempoolStream;
use crate::infrastructure::persistence::repositories::{
    MempoolSpendsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::utils::logging;

/// How often to poll the mempool (seconds)
const POLL_INTERVAL_SECS: u64 = 1;

/// Maximum number of streamed txs to process per poll cycle. Txs arrive
/// with their raw bytes, so this is CPU/DB-bound, not request-bound.
const MAX_TXS_PER_CYCLE: usize = 5_000;

/// Dedup cache cap: the feed doesn't replay, so this only guards against
/// overlap across reconnects; clear it once it gets large.
const MAX_SEEN_TXIDS: usize = 200_000;

/// How often to reload the monitored address set (every N cycles)
const MONITORED_SET_RELOAD_INTERVAL: u64 = 60;

/// How often to reconcile DB state with the live mempool (every N cycles).
/// At 1s per cycle, 30 cycles = every 30 seconds. The previous value of 5
/// minutes left a window where RBF-evicted txs stayed visible (audit N11).
const RECONCILE_INTERVAL_CYCLES: u64 = 30;

/// Cap on per-tx status lookups in one reconcile pass.
const MAX_RECONCILE_LOOKUPS: usize = 200;

/// Mempool processor — runs as a background task alongside the block processor
pub struct MempoolProcessor {
    bitcoin_client: BitcoinClient,
    stream: std::sync::Arc<MempoolStream>,
    db: DatabaseConnection,
    mempool_spends_repository: MempoolSpendsRepository,
    utxo_repository: UtxoRepository,
    monitored_addresses_repository: MonitoredAddressesRepository,
    network_id: NetworkId,
    seen_txids: std::sync::Arc<Mutex<HashSet<String>>>,
    monitored_set: std::sync::Arc<Mutex<HashSet<String>>>,
    /// Consecutive reconcile misses per pending txid. A tx is only evicted
    /// after the gateway stops knowing it for several reconcile cycles
    /// in a row, so transient snapshot blips don't flicker the explorer UI.
    reconcile_miss_counts: std::sync::Arc<Mutex<std::collections::HashMap<String, u32>>>,
}

impl MempoolProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        stream: std::sync::Arc<MempoolStream>,
        db: DatabaseConnection,
        mempool_spends_repository: MempoolSpendsRepository,
        utxo_repository: UtxoRepository,
        monitored_addresses_repository: MonitoredAddressesRepository,
        network_id: NetworkId,
    ) -> Self {
        Self {
            bitcoin_client,
            stream,
            db,
            mempool_spends_repository,
            utxo_repository,
            monitored_addresses_repository,
            network_id,
            seen_txids: std::sync::Arc::new(Mutex::new(HashSet::new())),
            monitored_set: std::sync::Arc::new(Mutex::new(HashSet::new())),
            reconcile_miss_counts: std::sync::Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Main loop — polls mempool every POLL_INTERVAL_SECS until `cancel` fires.
    pub async fn run(&self, cancel: tokio_util::sync::CancellationToken) {
        use tracing::Instrument;
        self.run_inner(cancel)
            .instrument(tracing::info_span!("mempool", network = %self.network_id.name))
            .await
    }

    async fn run_inner(&self, cancel: tokio_util::sync::CancellationToken) {
        logging::log_info(&format!(
            "🔍 MempoolProcessor started (poll every {}s)",
            POLL_INTERVAL_SECS
        ));

        let mut cycle: u64 = 0;
        self.reload_monitored_set().await;

        loop {
            if cancel.is_cancelled() {
                break;
            }
            cycle += 1;

            if cycle.is_multiple_of(MONITORED_SET_RELOAD_INTERVAL) {
                self.reload_monitored_set().await;
            }

            if let Err(e) = self.poll_once(cycle).await {
                logging::log_warning(&format!(
                    "[{}] ⚠️ MempoolProcessor cycle {} error: {}",
                    self.network_id.name, cycle, e
                ));
            }

            if cycle.is_multiple_of(100) {
                cleanup::purge_stale(
                    &self.network_id.name,
                    &self.db,
                    &self.mempool_spends_repository,
                    &self.seen_txids,
                )
                .await;
            }

            if cycle.is_multiple_of(RECONCILE_INTERVAL_CYCLES) {
                self.reconcile_with_mempool().await;
            }

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)) => {}
                _ = cancel.cancelled() => break,
            }
        }
        logging::log_info(&format!(
            "🛑 MempoolProcessor stopped after {} cycles",
            cycle
        ));
    }

    /// Single poll cycle: drain the websocket feed, detect charm txs, save them
    async fn poll_once(&self, cycle: u64) -> Result<(), String> {
        let streamed = self.stream.drain(MAX_TXS_PER_CYCLE).await;
        if streamed.is_empty() {
            if !self.stream.is_connected() && cycle.is_multiple_of(60) {
                return Err("mempool feed disconnected".to_string());
            }
            return Ok(());
        }

        let new_txs: Vec<(String, Option<String>)> = {
            let mut seen = self.seen_txids.lock().await;
            if seen.len() > MAX_SEEN_TXIDS {
                seen.clear();
            }
            streamed
                .into_iter()
                .filter(|t| seen.insert(t.txid.clone()))
                .map(|t| (t.txid, t.hex))
                .collect()
        };
        crate::utils::metrics::mempool_size(&self.network_id.name, new_txs.len());

        if new_txs.is_empty() {
            return Ok(());
        }

        logging::log_debug(&format!(
            "[{}] 🔍 Mempool cycle {}: {} new txs to check",
            self.network_id.name,
            cycle,
            new_txs.len()
        ));

        let mut charm_count = 0usize;
        let mut order_count = 0usize;

        // Get a snapshot of the monitored set for this cycle
        let monitored_snapshot = self.monitored_set.lock().await.clone();

        for (txid, hex) in &new_txs {
            // Track UTXOs for monitored addresses (ALL txs, not just charm txs).
            // The feed normally carries the raw tx; fetch only if it didn't.
            let fetched = match hex {
                Some(h) => Ok(h.clone()),
                None => self.bitcoin_client.get_raw_transaction_hex(txid).await,
            };
            let raw_hex = match fetched {
                Ok(hex) => hex,
                Err(e) => {
                    logging::log_debug(&format!(
                        "[{}] Mempool tx {} hex fetch failed: {}",
                        self.network_id.name, txid, e
                    ));
                    continue;
                }
            };

            // Deserialize once per tx and share it with every consumer below.
            // Previously the UTXO tracker, the vout-address extraction and the
            // spend extraction each re-parsed the same hex.
            let decoded = processor::decode_tx(&raw_hex);

            // Track UTXO changes for monitored addresses
            let mut spends_recorded = false;
            if !monitored_snapshot.is_empty() {
                if let Some(ref tx) = decoded {
                    spends_recorded = utxo_tracker::track_mempool_utxos(
                        txid,
                        tx,
                        &self.network_id.name,
                        &monitored_snapshot,
                        &self.utxo_repository,
                        &self.mempool_spends_repository,
                    )
                    .await;
                }
            }

            // Detect charms (pass raw_hex + decoded tx to avoid re-fetching/re-parsing)
            match processor::process_tx_with_hex(
                txid,
                &raw_hex,
                decoded.as_ref(),
                &self.network_id,
                &self.db,
                &self.mempool_spends_repository,
                spends_recorded,
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

    /// Reconcile DB with the gateway: revert all side effects for pending txs
    /// it no longer knows (dropped / RBF-replaced). Looks up each pending tx
    /// instead of pulling the full mempool listing (~5 MB on mainnet).
    async fn reconcile_with_mempool(&self) {
        let pending = match reconcile::get_pending_txids(&self.network_id.name, &self.db).await {
            Ok(p) => p,
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Reconcile: failed to fetch pending txids: {}",
                    self.network_id.name, e
                ));
                return;
            }
        };

        // A tx counts as live unless the gateway answers a definite 404;
        // lookup errors and txs past the lookup cap are given the benefit
        // of the doubt.
        let mut live_set: HashSet<String> = HashSet::new();
        for (i, txid) in pending.iter().enumerate() {
            let live = i >= MAX_RECONCILE_LOOKUPS
                || self.bitcoin_client.esplora().tx_exists(txid).await.unwrap_or(true);
            if live {
                live_set.insert(txid.clone());
            }
        }

        reconcile::reconcile_dropped_txs(
            &self.network_id.name,
            &live_set,
            &self.db,
            &self.reconcile_miss_counts,
        )
        .await;
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
