//! Charm detection from block transactions.
//! Uses TxAnalyzer for parsing, then adds block-specific logic
//! (supply calculation, metadata extraction, DEX order saving).

use bitcoincore_rpc::bitcoin;
use serde_json::json;
use std::collections::HashMap;

use crate::domain::services::dex;
use crate::domain::services::tx_analyzer::{self, AnalyzedTx};
use crate::domain::services::CharmService;
use crate::infrastructure::persistence::repositories::DexOrdersRepository;
use crate::utils::logging;

use super::batch::{AssetBatchItem, CharmBatchItem, TransactionBatchItem};

/// Detect charms from all transactions in a block.
/// Returns batch items for transactions, charms, and assets.
/// No DB writes except DEX order saving.
pub async fn detect_charms(
    block: &bitcoin::Block,
    height: u64,
    latest_height: u64,
    network: &str,
    blockchain: &str,
    charm_service: &CharmService,
    dex_repo: Option<&DexOrdersRepository>,
) -> (
    Vec<TransactionBatchItem>,
    Vec<CharmBatchItem>,
    Vec<AssetBatchItem>,
) {
    let tx_data = extract_transaction_data(block);

    let mut transaction_batch = Vec::new();
    let mut charm_batch = Vec::new();
    let mut asset_batch: Vec<AssetBatchItem> = Vec::new();

    for (txid, tx_hex, tx_pos, input_txids) in tx_data {
        let analyzed = match tx_analyzer::analyze_tx(&txid, &tx_hex, network) {
            Some(a) => a,
            None => continue,
        };

        let confirmations = latest_height - height + 1;

        // Log + save DEX orders
        if let Some(ref dex_res) = analyzed.dex_result {
            logging::log_info(&format!(
                "[{}] 🏷️ Block {}: Charms Cast DEX detected for tx {}: {:?}",
                network, height, txid, dex_res.operation
            ));

            if let (Some(ref order), Some(repo)) = (&dex_res.order, dex_repo) {
                match repo
                    .save_order(&txid, 0, Some(height), order, &dex_res.operation, "charms-cast", blockchain, network)
                    .await
                {
                    Ok(_) => logging::log_info(&format!(
                        "[{}] 💾 Block {}: Saved DEX order for tx {}: {:?} {:?}",
                        network, height, txid, order.side, dex_res.operation
                    )),
                    Err(e) => logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to save DEX order for tx {}: {}",
                        network, height, txid, e
                    )),
                }
            }
        }

        if analyzed.is_beaming {
            logging::log_info(&format!(
                "[{}] 🏷️ Block {}: Beaming transaction detected for tx {}",
                network, height, txid
            ));
        }

        if dex::is_bro_token(&analyzed.app_id) {
            logging::log_info(&format!(
                "[{}] 🏷️ Block {}: $BRO token detected for tx {}",
                network, height, txid
            ));
        }

        transaction_batch.push((
            txid.clone(),
            height,
            tx_pos as i64,
            json!({ "hex": tx_hex, "txid": txid }),
            analyzed.charm_json.clone(),
            confirmations as i32,
            true,
            blockchain.to_string(),
            network.to_string(),
        ));

        charm_batch.push((
            txid.clone(),
            0i32,
            height,
            analyzed.charm_json.clone(),
            analyzed.asset_type.clone(),
            blockchain.to_string(),
            network.to_string(),
            analyzed.address.clone(),
            analyzed.app_id.clone(),
            analyzed.amount,
            analyzed.tags.clone(),
        ));

        let asset_requests = build_asset_requests(
            &analyzed, &input_txids, height, blockchain, network, charm_service,
        )
        .await;
        asset_batch.extend(asset_requests);
    }

    (transaction_batch, charm_batch, asset_batch)
}

/// Build asset save requests from an analyzed tx.
/// Calculates net supply change (mint vs transfer) by comparing input/output amounts.
async fn build_asset_requests(
    analyzed: &AnalyzedTx,
    input_txids: &[String],
    height: u64,
    blockchain: &str,
    network: &str,
    charm_service: &CharmService,
) -> Vec<AssetBatchItem> {
    let input_amounts = if !input_txids.is_empty() {
        charm_service
            .get_charm_repository()
            .get_amounts_by_txids(input_txids)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    let mut net_changes: HashMap<String, i64> = HashMap::new();
    for asset in &analyzed.asset_infos {
        let nft_app_id = normalize_app_id(&asset.app_id, &asset.asset_type);
        *net_changes.entry(nft_app_id).or_insert(0) += asset.amount as i64;
    }

    for (_txid, app_id, amount) in &input_amounts {
        let nft_app_id = if app_id.starts_with("t/") {
            app_id.replacen("t/", "n/", 1)
        } else {
            app_id.clone()
        };
        *net_changes.entry(nft_app_id).or_insert(0) -= *amount as i64;
    }

    let metadata = extract_nft_metadata(analyzed);
    let (name, symbol, description, image_url, decimals) = parse_metadata_fields(&metadata);

    analyzed
        .asset_infos
        .iter()
        .filter_map(|asset| {
            let nft_app_id = normalize_app_id(&asset.app_id, &asset.asset_type);
            let net_change = net_changes.get(&nft_app_id).copied().unwrap_or(0);
            if net_change == 0 {
                return None;
            }

            let supply = net_change.max(0) as u64;
            let is_nft = asset.asset_type == "nft";

            Some((
                asset.app_id.clone(),
                analyzed.txid.clone(),
                0i32,
                height,
                asset.asset_type.clone(),
                supply,
                blockchain.to_string(),
                network.to_string(),
                if is_nft { name.clone() } else { None },
                if is_nft { symbol.clone() } else { None },
                if is_nft { description.clone() } else { None },
                if is_nft { image_url.clone() } else { None },
                if is_nft { decimals } else { None },
            ))
        })
        .collect()
}

fn normalize_app_id(app_id: &str, asset_type: &str) -> String {
    if asset_type == "token" {
        app_id.replacen("t/", "n/", 1)
    } else {
        app_id.to_string()
    }
}

fn extract_nft_metadata(analyzed: &AnalyzedTx) -> Option<serde_json::Value> {
    if analyzed.asset_type == "nft" {
        analyzed
            .charm_json
            .get("native_data")
            .and_then(|nd| nd.get("tx"))
            .and_then(|tx| tx.get("outs"))
            .and_then(|outs| outs.get(0))
            .and_then(|out0| out0.get("0"))
            .cloned()
    } else {
        None
    }
}

fn parse_metadata_fields(
    metadata: &Option<serde_json::Value>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u8>,
) {
    if let Some(ref meta) = metadata {
        (
            meta.get("name").and_then(|v| v.as_str()).map(String::from),
            meta.get("ticker")
                .or_else(|| meta.get("symbol"))
                .and_then(|v| v.as_str())
                .map(String::from),
            meta.get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            meta.get("image")
                .or_else(|| meta.get("url"))
                .or_else(|| meta.get("image_url"))
                .and_then(|v| v.as_str())
                .map(String::from),
            meta.get("decimals")
                .and_then(|v| v.as_u64())
                .map(|d| d as u8),
        )
    } else {
        (None, None, None, None, None)
    }
}

/// Extracts transaction data into an owned vector to avoid lifetime issues.
fn extract_transaction_data(
    block: &bitcoin::Block,
) -> Vec<(String, String, usize, Vec<String>)> {
    block
        .txdata
        .iter()
        .enumerate()
        .map(|(tx_pos, tx)| {
            let input_txids: Vec<String> = tx
                .input
                .iter()
                .filter(|input| !input.previous_output.is_null())
                .map(|input| input.previous_output.txid.to_string())
                .collect();

            (
                tx.txid().to_string(),
                bitcoin::consensus::encode::serialize_hex(tx),
                tx_pos,
                input_txids,
            )
        })
        .collect()
}
