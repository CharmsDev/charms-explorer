//! Mempool Processor — [RJJ-MEMPOOL]
//!
//! Polls Bitcoin Core's mempool every N seconds, detects charm transactions,
//! and saves them with block_height=NULL and mempool_detected_at=NOW().
//!
//! Detection is delegated to `tx_analyzer::analyze_tx` (shared with block processor).
//! This module only handles: fetching, persistence, and stale-entry purging.
//!
//! Design guarantees:
//! - NO stats_holders updates (only confirmed blocks update balances)
//! - NO asset supply updates (only confirmed blocks update supply)
//! - Duplicate-safe: ON CONFLICT DO NOTHING in all repositories
//! - Stale purge: removes mempool entries older than 24h

use std::collections::HashSet;
use std::time::Duration;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use tokio::sync::Mutex;

use crate::config::NetworkId;
use crate::domain::services::dex;
use crate::domain::services::tx_analyzer;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::entities::{charms, dex_orders};
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// How often to poll the mempool (seconds)
const POLL_INTERVAL_SECS: u64 = 1;

/// How many hours before a mempool entry is considered stale and purged
const STALE_HOURS: i64 = 24;

/// Maximum number of mempool txids to process per poll cycle
/// At 1s poll interval, 100 getrawtransaction calls/cycle is safe for the node.
/// New txids accumulate slowly; unseen ones are processed in subsequent cycles.
const MAX_TXS_PER_CYCLE: usize = 100;

/// Mempool processor — runs as a background task alongside the block processor
pub struct MempoolProcessor {
    bitcoin_client: BitcoinClient,
    db: DatabaseConnection,
    mempool_spends_repository: MempoolSpendsRepository,
    network_id: NetworkId,
    /// Set of txids already seen in mempool (avoids re-processing)
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

            // Purge stale entries every 100 cycles (~16 min)
            if cycle % 100 == 0 {
                self.purge_stale().await;
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    }

    /// Single poll cycle: fetch mempool, detect new charm txs, save them
    async fn poll_once(&self, cycle: u64) -> Result<(), String> {
        // 1. Get raw mempool txids from Bitcoin Core
        let mempool_txids = self.get_raw_mempool().await?;

        if mempool_txids.is_empty() {
            return Ok(());
        }

        // 2. Diff against seen set — acquire lock, collect new txids, release immediately.
        //    We mark them seen NOW (before processing) to avoid retry storms if processing
        //    is slow, but we release the lock before any RPC/DB work so we don't hold it
        //    for 10+ seconds and block the purge_stale path.
        let new_txids: Vec<String> = {
            let mut seen = self.seen_txids.lock().await;
            let new: Vec<String> = mempool_txids
                .into_iter()
                .filter(|txid| !seen.contains(txid))
                .take(MAX_TXS_PER_CYCLE)
                .collect();
            // Mark all as seen before releasing the lock
            for txid in &new {
                seen.insert(txid.clone());
            }
            new
        }; // lock released here

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

        // 3. Process each new txid — lock is NOT held here
        for txid in &new_txids {
            match self.process_mempool_tx(txid).await {
                Ok(Some(detected)) => {
                    charm_count += 1;
                    if detected.has_dex_order {
                        order_count += 1;
                    }
                }
                Ok(None) => {} // Not a charm tx — normal
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

    /// Process a single mempool transaction.
    /// Returns Some(result) if it's a charm tx, None if not.
    async fn process_mempool_tx(
        &self,
        txid: &str,
    ) -> Result<Option<MempoolDetectionResult>, String> {
        let raw_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, None)
            .await
            .map_err(|e| format!("get_raw_transaction_hex failed: {}", e))?;

        // Analyze tx using shared TxAnalyzer (CPU-intensive, run in blocking task)
        let txid_owned = txid.to_string();
        let raw_hex_clone = raw_hex.clone();
        let network = self.network_id.name.clone();
        let analyzed = tokio::task::spawn_blocking(move || {
            tx_analyzer::analyze_tx(&txid_owned, &raw_hex_clone, &network)
        })
        .await
        .map_err(|e| format!("spawn_blocking join error: {}", e))?;

        let analyzed = match analyzed {
            Some(a) => a,
            None => return Ok(None), // Not a charm tx
        };

        let network = self.network_id.name.clone();
        let blockchain = "Bitcoin".to_string();
        let now = Utc::now().naive_utc();
        let has_dex_order = analyzed
            .dex_result
            .as_ref()
            .map_or(false, |d| d.order.is_some());

        // Log DEX detection
        if let Some(ref dex) = analyzed.dex_result {
            logging::log_info(&format!(
                "[{}] 🏷️ Mempool: Charms Cast DEX detected for tx {}: {:?}",
                network, txid, dex.operation
            ));
        }

        // Save charm with block_height=NULL (mempool)
        let charm_model = charms::ActiveModel {
            txid: Set(txid.to_string()),
            vout: Set(0i32),
            block_height: Set(None),
            data: Set(analyzed.charm_json.clone()),
            date_created: Set(now),
            asset_type: Set(analyzed.asset_type.clone()),
            blockchain: Set(blockchain.clone()),
            network: Set(network.clone()),
            address: Set(analyzed.address.clone()),
            spent: Set(false),
            app_id: Set(analyzed.app_id.clone()),
            amount: Set(analyzed.amount),
            mempool_detected_at: Set(Some(now)),
            tags: Set(analyzed.tags.clone()),
            verified: Set(true),
        };

        match charm_model.insert(&self.db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 💾 Mempool charm saved: {} ({})",
                    network, txid, analyzed.asset_type
                ));
            }
            Err(e) if e.to_string().contains("duplicate key") => {}
            Err(e) => {
                return Err(format!("Failed to save mempool charm: {}", e));
            }
        }

        // Save DEX order with block_height=NULL
        self.save_mempool_dex_order(txid, &analyzed, &blockchain, &network)
            .await;

        // Record mempool spends (inputs being consumed by this tx)
        let spends = self.extract_mempool_spends(&raw_hex, txid);
        if !spends.is_empty() {
            if let Err(e) = self
                .mempool_spends_repository
                .record_spends_batch(&spends, &network)
                .await
            {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Failed to record mempool spends for {}: {}",
                    network, txid, e
                ));
            }
        }

        Ok(Some(MempoolDetectionResult { has_dex_order }))
    }

    /// Save a DEX order detected in a mempool transaction
    async fn save_mempool_dex_order(
        &self,
        txid: &str,
        analyzed: &tx_analyzer::AnalyzedTx,
        blockchain: &str,
        network: &str,
    ) {
        let dex_result = match &analyzed.dex_result {
            Some(d) => d,
            None => return,
        };
        let order = match &dex_result.order {
            Some(o) => o,
            None => return,
        };

        let order_id = format!("{}:0", txid);
        let now_dt = Utc::now().naive_utc();

        let status = match dex_result.operation {
            dex::DexOperation::CreateAskOrder | dex::DexOperation::CreateBidOrder => "open",
            dex::DexOperation::PartialFill => "partial",
            dex::DexOperation::FulfillAsk | dex::DexOperation::FulfillBid => "filled",
            dex::DexOperation::CancelOrder => "cancelled",
        };

        use crate::domain::services::dex::{ExecType, OrderSide};
        let side_str = match order.side {
            OrderSide::Ask => "ask",
            OrderSide::Bid => "bid",
        };
        let exec_type_str = match &order.exec_type {
            ExecType::AllOrNone => "all_or_none",
            ExecType::Partial { .. } => "partial",
        };
        let parent_order_id = if let ExecType::Partial { from } = &order.exec_type {
            from.clone()
        } else {
            None
        };

        let order_model = dex_orders::ActiveModel {
            order_id: Set(order_id),
            txid: Set(txid.to_string()),
            vout: Set(0i32),
            block_height: Set(None),
            platform: Set("charms-cast".to_string()),
            maker: Set(order.maker.clone()),
            side: Set(side_str.to_string()),
            exec_type: Set(exec_type_str.to_string()),
            price_num: Set(order.price.0 as i64),
            price_den: Set(order.price.1 as i64),
            amount: Set(order.amount as i64),
            quantity: Set(order.quantity as i64),
            filled_amount: Set(0),
            filled_quantity: Set(0),
            asset_app_id: Set(order.asset_app_id.clone()),
            scrolls_address: Set(order.scrolls_address.clone()),
            status: Set(status.to_string()),
            parent_order_id: Set(parent_order_id),
            created_at: Set(now_dt),
            updated_at: Set(now_dt),
            blockchain: Set(blockchain.to_string()),
            network: Set(network.to_string()),
        };

        match order_model.insert(&self.db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 💾 Mempool DEX order saved: {} ({:?})",
                    network, txid, dex_result.operation
                ));
            }
            Err(e) if e.to_string().contains("duplicate key") => {}
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Failed to save mempool DEX order {}: {}",
                    network, txid, e
                ));
            }
        }
    }

    /// Extract (spending_txid, spent_txid, spent_vout) from a raw tx hex
    fn extract_mempool_spends(
        &self,
        raw_hex: &str,
        spending_txid: &str,
    ) -> Vec<(String, String, i32)> {
        use bitcoin::consensus::deserialize;

        let tx_bytes = match hex::decode(raw_hex) {
            Ok(b) => b,
            Err(_) => return vec![],
        };

        let tx: bitcoin::Transaction = match deserialize(&tx_bytes) {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        tx.input
            .iter()
            .filter_map(|inp| {
                let prev_txid = inp.previous_output.txid.to_string();
                let prev_vout = inp.previous_output.vout as i32;
                // Skip coinbase inputs (all zeros txid)
                if prev_txid == "0000000000000000000000000000000000000000000000000000000000000000" {
                    None
                } else {
                    Some((spending_txid.to_string(), prev_txid, prev_vout))
                }
            })
            .collect()
    }

    /// Fetch raw mempool txids via Bitcoin Core RPC
    async fn get_raw_mempool(&self) -> Result<Vec<String>, String> {
        self.bitcoin_client
            .get_raw_mempool()
            .await
            .map_err(|e| format!("getrawmempool failed: {}", e))
    }

    /// Purge stale mempool entries (charms/orders/spends older than STALE_HOURS)
    async fn purge_stale(&self) {
        let network = &self.network_id.name;

        // Purge stale mempool_spends
        match self
            .mempool_spends_repository
            .purge_stale(STALE_HOURS)
            .await
        {
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

        // Purge stale mempool charms (block_height IS NULL AND mempool_detected_at < NOW() - 24h)
        let sql = format!(
            "DELETE FROM charms WHERE block_height IS NULL AND network = '{}' \
             AND mempool_detected_at < NOW() - INTERVAL '{} hours'",
            network, STALE_HOURS
        );
        use sea_orm::{ConnectionTrait, DbBackend, Statement};
        match self
            .db
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

        // Purge stale mempool DEX orders
        let sql_orders = format!(
            "DELETE FROM dex_orders WHERE block_height IS NULL AND network = '{}' \
             AND created_at < NOW() - INTERVAL '{} hours'",
            network, STALE_HOURS
        );
        let _ = self
            .db
            .execute(Statement::from_string(DbBackend::Postgres, sql_orders))
            .await;

        // Also clear seen_txids set periodically to avoid unbounded growth
        let mut seen = self.seen_txids.lock().await;
        if seen.len() > 10_000 {
            seen.clear();
            logging::log_info(&format!(
                "[{}] 🧹 Cleared seen_txids cache (was >10k entries)",
                network
            ));
        }
    }
}

/// Result of processing a single mempool tx
struct MempoolDetectionResult {
    has_dex_order: bool,
}
