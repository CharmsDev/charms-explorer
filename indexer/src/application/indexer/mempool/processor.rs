//! Core mempool transaction processing: detect charm txs and save them.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::config::NetworkId;
use crate::domain::services::dex;
use crate::domain::services::tx_analyzer;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::entities::{charms, dex_orders};
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
    let blockchain = "Bitcoin".to_string();
    let now = Utc::now().naive_utc();
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

    match charm_model.insert(db).await {
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
    save_dex_order(txid, &analyzed, &blockchain, &network, db).await;

    // Record mempool spends (inputs being consumed by this tx)
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
