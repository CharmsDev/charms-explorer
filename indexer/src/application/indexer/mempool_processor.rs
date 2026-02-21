//! Mempool Processor — [RJJ-MEMPOOL]
//!
//! Polls Bitcoin Core's mempool every N seconds, detects charm transactions,
//! and saves them with block_height=NULL and mempool_detected_at=NOW().
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
use crate::domain::services::address_extractor::AddressExtractor;
use crate::domain::services::dex;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::entities::{charms, dex_orders};
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// How often to poll the mempool (seconds)
const POLL_INTERVAL_SECS: u64 = 10;

/// How many hours before a mempool entry is considered stale and purged
const STALE_HOURS: i64 = 24;

/// Maximum number of mempool txids to process per poll cycle
/// (avoids overwhelming the node with getrawtransaction calls)
const MAX_TXS_PER_CYCLE: usize = 500;

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

        // 2. Filter out already-seen txids
        let mut seen = self.seen_txids.lock().await;
        let new_txids: Vec<String> = mempool_txids
            .into_iter()
            .filter(|txid| !seen.contains(txid))
            .take(MAX_TXS_PER_CYCLE)
            .collect();

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

        // 3. Process each new txid
        for txid in &new_txids {
            // Mark as seen immediately (even if processing fails, avoid retry storm)
            seen.insert(txid.clone());

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
        // Fetch raw tx hex from node (mempool tx, no block_hash needed)
        let raw_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, None)
            .await
            .map_err(|e| format!("get_raw_transaction_hex failed: {}", e))?;

        // Try to detect a charm spell in this tx (CPU-intensive, run in blocking task)
        let raw_hex_owned = raw_hex.clone();
        let parse_result = tokio::task::spawn_blocking(move || {
            NativeCharmParser::extract_and_verify_charm(&raw_hex_owned, false)
        })
        .await
        .map_err(|e| format!("spawn_blocking join error: {}", e))?;

        let spell = match parse_result {
            Ok(s) => s,
            Err(_) => return Ok(None), // Not a charm tx
        };

        let network = self.network_id.name.clone();
        let blockchain = "Bitcoin".to_string();
        let now = Utc::now().naive_utc();

        // Serialize spell to JSON for storage
        let charm_json = serde_json::json!({
            "type": "spell",
            "detected": true,
            "has_native_data": true,
            "native_data": serde_json::to_value(&spell).unwrap_or_default(),
            "version": "native_parser"
        });

        // Extract asset info
        let asset_infos = NativeCharmParser::extract_asset_info(&spell);
        let (app_id, asset_type, amount) = if let Some(first) = asset_infos.first() {
            let atype = if first.app_id.starts_with("t/") {
                "token"
            } else if first.app_id.starts_with("n/") {
                "nft"
            } else if first.app_id.starts_with("B/") {
                "dapp"
            } else {
                "other"
            };
            (first.app_id.clone(), atype.to_string(), first.amount as i64)
        } else {
            ("other".to_string(), "spell".to_string(), 0i64)
        };

        // Extract holder address
        let address = AddressExtractor::extract_charm_holder_address(&raw_hex, &network)
            .ok()
            .flatten();

        // Detect DEX operation
        let dex_result = dex::detect_dex_operation(&charm_json);
        let mut tags: Vec<String> = Vec::new();
        let mut has_dex_order = false;

        if let Some(ref result) = dex_result {
            tags.push("charms-cast".to_string());
            tags.push(result.operation.to_tag().to_string());
            has_dex_order = result.order.is_some();

            logging::log_info(&format!(
                "[{}] 🏷️ Mempool: Charms Cast DEX detected for tx {}: {:?}",
                network, txid, result.operation
            ));
        }

        if dex::is_bro_token(&app_id) {
            tags.push("bro".to_string());
        }

        let tags_str = if tags.is_empty() {
            None
        } else {
            Some(tags.join(","))
        };

        // Save charm with block_height=NULL (mempool)
        let charm_model = charms::ActiveModel {
            txid: Set(txid.to_string()),
            vout: Set(0i32),
            block_height: Set(None), // NULL = mempool, not yet confirmed
            data: Set(charm_json.clone()),
            date_created: Set(now),
            asset_type: Set(asset_type.clone()),
            blockchain: Set(blockchain.clone()),
            network: Set(network.clone()),
            address: Set(address.clone()),
            spent: Set(false),
            app_id: Set(app_id.clone()),
            amount: Set(amount),
            mempool_detected_at: Set(Some(now)),
            tags: Set(tags_str.clone()),
            verified: Set(true),
        };

        match charm_model.insert(&self.db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 💾 Mempool charm saved: {} ({})",
                    network, txid, asset_type
                ));
            }
            Err(e) if e.to_string().contains("duplicate key") => {
                // Already indexed (from a previous cycle or block) — OK
            }
            Err(e) => {
                return Err(format!("Failed to save mempool charm: {}", e));
            }
        }

        // Save DEX order with block_height=NULL
        if let Some(ref dex) = dex_result {
            if let Some(ref order) = dex.order {
                let order_id = format!("{}:0", txid);
                let now_dt = Utc::now().naive_utc();

                let status = match dex.operation {
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
                    block_height: Set(None), // NULL = mempool
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
                    blockchain: Set(blockchain.clone()),
                    network: Set(network.clone()),
                };

                match order_model.insert(&self.db).await {
                    Ok(_) => {
                        logging::log_info(&format!(
                            "[{}] 💾 Mempool DEX order saved: {} ({:?})",
                            network, txid, dex.operation
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
        }

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
