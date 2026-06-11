//! Core mempool transaction processing: detect charm txs and save them.
//!
//! Persistence of DEX orders / activity rows lives in `dex_persistence`
//! and the consumed-UTXO extraction lives in `spend_extraction`.

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use super::dex_persistence::{
    correct_fulfill_classification, save_dex_order, update_consumed_order_status,
};
use super::spend_extraction::extract_spends;
use crate::config::NetworkId;
use crate::domain::services::tx_analyzer;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::entities::{charms, transactions};
use crate::infrastructure::persistence::error::is_duplicate_key;
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
    // stats_holders is NOT updated here — it only tracks confirmed balances.
    // Unconfirmed balance is computed at query time from charms WHERE block_height IS NULL.
    for asset in &analyzed.asset_infos {
        let address = vout_addresses
            .get(asset.vout_index as usize)
            .and_then(|a| a.clone());
        let is_beamed_out = analyzed.beamed_out_indices.contains(&(asset.vout_index as usize));
        let charm_model = charms::ActiveModel {
            txid: Set(txid.to_string()),
            vout: Set(asset.vout_index),
            block_height: Set(None),
            data: Set(analyzed.charm_json.clone()),
            date_created: Set(now),
            asset_type: Set(asset.asset_type.clone()),
            blockchain: Set(blockchain.clone()),
            network: Set(network.clone()),
            address: Set(address),
            spent: Set(false),
            app_id: Set(asset.app_id.clone()),
            amount: Set(if is_beamed_out { 0i64 } else { asset.amount as i64 }),
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
            }
            Err(e) if is_duplicate_key(&e) => {}
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
        tx_type: Set(Some(analyzed.tx_type.clone())),
    };
    match tx_model.insert(db).await {
        Ok(_) => {}
        Err(e) if is_duplicate_key(&e) => {}
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
    // stats_holders is NOT updated here — spent tracking only happens at block confirmation
    // via spent_tracker::mark_spent_charms to avoid double-subtraction.
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
