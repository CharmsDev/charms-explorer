//! Core mempool transaction processing: detect charm txs and save them.

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Set, Statement};

use crate::config::NetworkId;
use crate::domain::services::dex::{self, extract_ins0_order_id};
use crate::domain::services::tx_analyzer;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::entities::{charms, dex_orders, transactions};
use crate::infrastructure::persistence::repositories::MempoolSpendsRepository;
use crate::utils::logging;

/// Result of processing a single mempool tx
pub struct MempoolDetectionResult {
    pub has_dex_order: bool,
}

/// Process a single mempool transaction (fetches raw hex internally).
/// Returns Some(result) if it's a charm tx, None if not.
#[allow(dead_code)]
pub async fn process_tx(
    txid: &str,
    network_id: &NetworkId,
    bitcoin_client: &BitcoinClient,
    db: &DatabaseConnection,
    mempool_spends_repository: &MempoolSpendsRepository,
) -> Result<Option<MempoolDetectionResult>, String> {
    let raw_hex = bitcoin_client
        .get_raw_transaction_hex(txid, None)
        .await
        .map_err(|e| format!("get_raw_transaction_hex failed: {}", e))?;

    process_tx_with_hex(txid, &raw_hex, network_id, db, mempool_spends_repository).await
}

/// Process a single mempool transaction with pre-fetched raw hex.
/// Returns Some(result) if it's a charm tx, None if not.
pub async fn process_tx_with_hex(
    txid: &str,
    raw_hex: &str,
    network_id: &NetworkId,
    db: &DatabaseConnection,
    mempool_spends_repository: &MempoolSpendsRepository,
) -> Result<Option<MempoolDetectionResult>, String> {
    // Analyze tx using shared TxAnalyzer (CPU-intensive, run in blocking task)
    let txid_owned = txid.to_string();
    let raw_hex_clone = raw_hex.to_string();
    let network = network_id.name.clone();
    let analyzed = tokio::task::spawn_blocking(move || {
        tx_analyzer::analyze_tx(&txid_owned, &raw_hex_clone, &network)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))?;

    let analyzed = match analyzed {
        Some(a) => a,
        None => return Ok(None),
    };

    let network = network_id.name.clone();

    // [FULFILL-BID correction] detect_dex_operation() returns FulfillAsk for all 3-output
    // fulfills because the spell structure is identical for FULFILL-ASK and FULFILL-BID
    // without token change. Look up the consumed order in dex_orders to disambiguate.
    let analyzed = correct_fulfill_classification(txid, raw_hex, analyzed, &network, db).await;
    let blockchain = "Bitcoin".to_string();
    let now = Utc::now().naive_utc();
    let now_tz: DateTime<FixedOffset> = Utc::now().fixed_offset();
    let has_dex_order = analyzed
        .dex_result
        .as_ref()
        .map_or(false, |d| d.order.is_some());

    if let Some(ref dex) = analyzed.dex_result {
        logging::log_info(&format!(
            "[{}] 🏷️ Mempool: Charms Cast DEX detected for tx {}: {:?}",
            network, txid, dex.operation
        ));
    }

    // Extract per-vout addresses (preserving index alignment, OP_RETURN outputs map to None)
    let vout_addresses: Vec<Option<String>> = {
        use bitcoincore_rpc::bitcoin::{consensus::deserialize, Address, Network, Transaction};
        let btc_network = match network.as_str() {
            "mainnet" => Network::Bitcoin,
            "testnet4" | "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Testnet,
        };
        hex::decode(raw_hex)
            .ok()
            .and_then(|bytes| deserialize::<Transaction>(&bytes).ok())
            .map(|tx| {
                tx.output
                    .iter()
                    .map(|out| {
                        Address::from_script(&out.script_pubkey, btc_network)
                            .ok()
                            .map(|a| a.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    // Save one charm entry per charm-bearing output with block_height=NULL (mempool)
    // and update stats_holders for incoming charms
    for asset in &analyzed.asset_infos {
        let address = vout_addresses
            .get(asset.vout_index as usize)
            .and_then(|a| a.clone());
        let charm_model = charms::ActiveModel {
            txid: Set(txid.to_string()),
            vout: Set(asset.vout_index),
            block_height: Set(None),
            data: Set(analyzed.charm_json.clone()),
            date_created: Set(now),
            asset_type: Set(asset.asset_type.clone()),
            blockchain: Set(blockchain.clone()),
            network: Set(network.clone()),
            address: Set(address.clone()),
            spent: Set(false),
            app_id: Set(asset.app_id.clone()),
            amount: Set(asset.amount as i64),
            mempool_detected_at: Set(Some(now_tz)),
            tags: Set(analyzed.tags.clone()),
            verified: Set(true),
        };
        match charm_model.insert(db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 💾 Mempool charm saved: {} vout={} ({})",
                    network, txid, asset.vout_index, asset.asset_type
                ));

                // Update stats_holders for the new mempool charm
                if let Some(ref addr) = address {
                    if !addr.is_empty() && asset.amount > 0 {
                        let (holder_app_id, delta) = if asset.app_id.starts_with("t/") {
                            (asset.app_id.replacen("t/", "n/", 1), asset.amount as i64)
                        } else if asset.app_id.starts_with("n/") {
                            (asset.app_id.clone(), 1_i64)
                        } else {
                            (asset.app_id.clone(), asset.amount as i64)
                        };
                        update_stats_holders_raw(db, &holder_app_id, addr, delta, 0).await;
                    }
                }
            }
            Err(e) if e.to_string().contains("duplicate key") => {}
            Err(e) => {
                return Err(format!("Failed to save mempool charm: {}", e));
            }
        }
    }

    // Save transaction with block_height=NULL and status='pending' (mempool)
    let raw_json = serde_json::json!({"hex": raw_hex});
    let tx_model = transactions::ActiveModel {
        txid: Set(txid.to_string()),
        block_height: Set(None),
        ordinal: Set(0i64),
        raw: Set(raw_json),
        charm: Set(analyzed.charm_json.clone()),
        updated_at: Set(now),
        status: Set("pending".to_string()),
        confirmations: Set(0i32),
        blockchain: Set(blockchain.clone()),
        network: Set(network.clone()),
        mempool_detected_at: Set(Some(now_tz)),
        tags: Set(analyzed.tags.clone()),
    };
    match tx_model.insert(db).await {
        Ok(_) => {}
        Err(e) if e.to_string().contains("duplicate key") => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to save mempool transaction {}: {}",
                network, txid, e
            ));
        }
    }

    // Save DEX order with block_height=NULL (only for CREATE operations)
    save_dex_order(txid, &analyzed, &blockchain, &network, db).await;

    // Update the consumed order status for FULFILL and CANCEL operations
    // and insert an activity row for the fulfill/cancel transaction
    update_consumed_order_status(txid, raw_hex, &analyzed, &blockchain, &network, db).await;

    // Record mempool spends (inputs being consumed by this tx)
    // and update stats_holders for spent charm inputs (subtract balance)
    let spends = extract_spends(&raw_hex, txid);
    if !spends.is_empty() {
        if let Err(e) = mempool_spends_repository
            .record_spends_batch(&spends, &network)
            .await
        {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to record mempool spends for {}: {}",
                network, txid, e
            ));
        }

        // Check if any spent inputs are charm UTXOs and subtract from stats_holders
        let spent_pairs: Vec<String> = spends
            .iter()
            .map(|(_, spent_txid, vout)| format!("('{}', {})", spent_txid.replace('\'', "''"), vout))
            .collect();
        let sql = format!(
            "SELECT app_id, address, amount FROM charms WHERE (txid, vout) IN (VALUES {}) AND spent = false AND address IS NOT NULL",
            spent_pairs.join(", ")
        );
        if let Ok(rows) = db.query_all(Statement::from_string(DbBackend::Postgres, sql)).await {
            for row in &rows {
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
                    update_stats_holders_raw(db, &holder_app_id, &addr, delta, 0).await;
                }
            }
        }
    }

    Ok(Some(MempoolDetectionResult { has_dex_order }))
}

/// Save a DEX order detected in a mempool transaction
async fn save_dex_order(
    txid: &str,
    analyzed: &tx_analyzer::AnalyzedTx,
    blockchain: &str,
    network: &str,
    db: &DatabaseConnection,
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

    match order_model.insert(db).await {
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

/// Correct FULFILL-BID misclassified as FulfillAsk (3-output edge case).
///
/// detect_dex_operation() returns FulfillAsk for all 3-output fulfills because
/// FULFILL-ASK and FULFILL-BID without token change have identical spell structures.
/// This function looks up the consumed order (ins[0]) in dex_orders and, if its
/// side is "bid", corrects the operation and tag to FulfillBid.
async fn correct_fulfill_classification(
    txid: &str,
    raw_hex: &str,
    mut analyzed: tx_analyzer::AnalyzedTx,
    network: &str,
    db: &DatabaseConnection,
) -> tx_analyzer::AnalyzedTx {
    let is_fulfill_ask = analyzed
        .dex_result
        .as_ref()
        .map_or(false, |d| d.operation == dex::DexOperation::FulfillAsk);
    if !is_fulfill_ask {
        return analyzed;
    }

    let order_id = match extract_ins0_order_id(raw_hex) {
        Some(id) => id,
        None => return analyzed,
    };

    let order = match dex_orders::Entity::find_by_id(order_id.clone())
        .one(db)
        .await
    {
        Ok(Some(o)) => o,
        _ => return analyzed, // Not found or DB error → keep FulfillAsk
    };

    if order.side == "bid" {
        if let Some(ref mut result) = analyzed.dex_result {
            result.operation = dex::DexOperation::FulfillBid;
        }
        if let Some(ref mut tags) = analyzed.tags {
            *tags = tags.replace("fulfill-ask", "fulfill-bid");
        }
        logging::log_info(&format!(
            "[{}] 🔄 FULFILL-BID (3-out) corrected for tx {} (consumed order {})",
            network, txid, order_id
        ));
    }

    analyzed
}

/// Update the consumed order's status in dex_orders when a FULFILL or CANCEL
/// is detected in the mempool. Also inserts a new activity row for the
/// fulfill/cancel transaction, copying data from the parent order.
async fn update_consumed_order_status(
    txid: &str,
    raw_hex: &str,
    analyzed: &tx_analyzer::AnalyzedTx,
    blockchain: &str,
    network: &str,
    db: &DatabaseConnection,
) {
    let new_status = match analyzed.dex_result.as_ref().map(|d| &d.operation) {
        Some(dex::DexOperation::FulfillAsk) | Some(dex::DexOperation::FulfillBid) => "filled",
        Some(dex::DexOperation::CancelOrder) => "cancelled",
        _ => return,
    };

    let order_id = match extract_ins0_order_id(raw_hex) {
        Some(id) => id,
        None => return,
    };

    let order = match dex_orders::Entity::find_by_id(order_id.clone())
        .one(db)
        .await
    {
        Ok(Some(o)) => o,
        Ok(None) => return, // Order not yet indexed (e.g., still in mempool)
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to look up order {} for status update: {}",
                network, order_id, e
            ));
            return;
        }
    };

    // Insert activity row for the fulfill/cancel transaction
    let activity_order_id = format!("{}:0", txid);
    let now = chrono::Utc::now().naive_utc();
    let activity_model = dex_orders::ActiveModel {
        order_id: Set(activity_order_id.clone()),
        txid: Set(txid.to_string()),
        vout: Set(0i32),
        block_height: Set(None), // mempool
        platform: Set(order.platform.clone()),
        maker: Set(order.maker.clone()),
        side: Set(order.side.clone()),
        exec_type: Set(order.exec_type.clone()),
        price_num: Set(order.price_num),
        price_den: Set(order.price_den),
        amount: Set(order.amount),
        quantity: Set(order.quantity),
        filled_amount: Set(0),
        filled_quantity: Set(0),
        asset_app_id: Set(order.asset_app_id.clone()),
        scrolls_address: Set(order.scrolls_address.clone()),
        status: Set(new_status.to_string()),
        parent_order_id: Set(Some(order.order_id.clone())),
        created_at: Set(now),
        updated_at: Set(now),
        blockchain: Set(blockchain.to_string()),
        network: Set(network.to_string()),
    };

    match activity_model.insert(db).await {
        Ok(_) => {
            logging::log_info(&format!(
                "[{}] 💾 Mempool activity row saved: {} ({}) parent={}",
                network, txid, new_status, order_id
            ));
        }
        Err(e) if e.to_string().contains("duplicate key") => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to save activity row for {}: {}",
                network, txid, e
            ));
        }
    }

    // Update parent order status
    if order.status == "open" || order.status == "partial" {
        let mut active: dex_orders::ActiveModel = order.into();
        active.status = Set(new_status.to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        match active.update(db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 🔄 Order {} → {} (mempool)",
                    network, order_id, new_status
                ));
            }
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Failed to update order {} status to {}: {}",
                    network, order_id, new_status, e
                ));
            }
        }
    }
}

/// Update stats_holders directly via raw SQL (used by mempool processor
/// which doesn't have access to StatsHoldersRepository).
async fn update_stats_holders_raw(db: &DatabaseConnection, app_id: &str, address: &str, delta: i64, block_height: i32) {
    let sql = format!(
        r#"INSERT INTO stats_holders (app_id, address, total_amount, charm_count, first_seen_block, last_updated_block, created_at, updated_at)
           VALUES ('{}', '{}', {}, 1, {}, {}, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
           ON CONFLICT (app_id, address) DO UPDATE SET
               total_amount = stats_holders.total_amount + {},
               charm_count = CASE WHEN {} > 0 THEN stats_holders.charm_count + 1 ELSE stats_holders.charm_count - 1 END,
               last_updated_block = {},
               updated_at = CURRENT_TIMESTAMP"#,
        app_id.replace('\'', "''"),
        address.replace('\'', "''"),
        delta, block_height, block_height,
        delta, delta, block_height
    );
    if let Err(e) = db.execute(Statement::from_string(DbBackend::Postgres, sql)).await {
        logging::log_warning(&format!(
            "⚠️ Failed to update stats_holders for {}/{}: {}", app_id, address, e
        ));
    }

    // Clean up zero-balance holders
    if delta < 0 {
        let cleanup_sql = format!(
            "DELETE FROM stats_holders WHERE app_id = '{}' AND address = '{}' AND total_amount <= 0",
            app_id.replace('\'', "''"),
            address.replace('\'', "''")
        );
        let _ = db.execute(Statement::from_string(DbBackend::Postgres, cleanup_sql)).await;
    }
}

// extract_ins0_order_id is now in crate::domain::services::dex::extract_ins0_order_id

/// Extract (spending_txid, spent_txid, spent_vout) from a raw tx hex
fn extract_spends(raw_hex: &str, spending_txid: &str) -> Vec<(String, String, i32)> {
    use bitcoincore_rpc::bitcoin::{self, consensus::deserialize};

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
            if prev_txid == "0000000000000000000000000000000000000000000000000000000000000000" {
                None
            } else {
                Some((spending_txid.to_string(), prev_txid, prev_vout))
            }
        })
        .collect()
}
