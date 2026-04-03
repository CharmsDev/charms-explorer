//! Charm detection from block transactions.
//! Uses TxAnalyzer for parsing, then adds block-specific logic
//! (supply calculation, metadata extraction, DEX order saving).

use bitcoincore_rpc::bitcoin;
use serde_json::json;
use std::collections::HashMap;

use crate::domain::services::dex::{self, extract_ins0_order_id};
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

            if let Some(repo) = dex_repo {
                if let Some(ref order) = dex_res.order {
                    // CREATE or PARTIAL: save the order directly
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
                } else {
                    // FULFILL or CANCEL: no order in outputs, insert activity row from parent
                    let new_status = match dex_res.operation {
                        dex::DexOperation::FulfillAsk | dex::DexOperation::FulfillBid => "filled",
                        dex::DexOperation::CancelOrder => "cancelled",
                        _ => "unknown",
                    };
                    if let Some(parent_order_id) = extract_ins0_order_id(&tx_hex) {
                        match repo.get_by_id(&parent_order_id).await {
                            Ok(Some(parent)) => {
                                match repo.save_activity_row(&txid, Some(height), &parent, new_status, blockchain, network).await {
                                    Ok(_) => logging::log_info(&format!(
                                        "[{}] 💾 Block {}: Saved activity row for tx {} ({}) parent={}",
                                        network, height, txid, new_status, parent_order_id
                                    )),
                                    Err(e) => logging::log_error(&format!(
                                        "[{}] ❌ Block {}: Failed to save activity row for tx {}: {}",
                                        network, height, txid, e
                                    )),
                                }
                                // Update parent order status
                                if let Err(e) = repo.update_status(&parent_order_id, new_status).await {
                                    logging::log_warning(&format!(
                                        "[{}] ⚠️ Block {}: Failed to update parent order {} status: {}",
                                        network, height, parent_order_id, e
                                    ));
                                }
                            }
                            Ok(None) => logging::log_warning(&format!(
                                "[{}] ⚠️ Block {}: Parent order {} not found for tx {}",
                                network, height, parent_order_id, txid
                            )),
                            Err(e) => logging::log_error(&format!(
                                "[{}] ❌ Block {}: Failed to look up parent order {}: {}",
                                network, height, parent_order_id, e
                            )),
                        }
                    }
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
            analyzed.tags.clone(),
            Some(analyzed.tx_type.clone()),
        ));

        // Extract per-vout addresses (preserving index alignment, OP_RETURN outputs map to None)
        let vout_addresses: Vec<Option<String>> = {
            use bitcoincore_rpc::bitcoin::{consensus::deserialize, Address, Network, Transaction};
            let btc_network = match network {
                "mainnet" => Network::Bitcoin,
                "testnet4" | "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => Network::Testnet,
            };
            hex::decode(&tx_hex)
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

        // Push one charm entry per charm-bearing output with its correct vout
        for asset in &analyzed.asset_infos {
            let address = vout_addresses
                .get(asset.vout_index as usize)
                .and_then(|a| a.clone());
            charm_batch.push((
                txid.clone(),
                asset.vout_index,
                height,
                analyzed.charm_json.clone(),
                asset.asset_type.clone(),
                blockchain.to_string(),
                network.to_string(),
                address,
                asset.app_id.clone(),
                asset.amount as i64,
                analyzed.tags.clone(),
            ));
        }

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
    let (mut name, mut symbol, mut description, mut image_url, mut decimals) =
        parse_metadata_fields(&metadata);

    // Enrich beaming assets with Cardano token metadata
    let mut cardano_policy_id: Option<String> = None;
    let mut cardano_asset_name: Option<String> = None;
    let mut cardano_fingerprint: Option<String> = None;

    if analyzed.is_beaming {
        if let Some(cardano_meta) = fetch_cardano_metadata_for_beaming(analyzed).await {
            // Always store Cardano IDs for beaming transactions
            cardano_policy_id = Some(cardano_meta.policy_id);
            cardano_asset_name = Some(cardano_meta.asset_name_hex);
            cardano_fingerprint = Some(cardano_meta.fingerprint);

            // Fill metadata gaps from Cardano (don't overwrite existing)
            if name.is_none() { name = cardano_meta.name; }
            if symbol.is_none() { symbol = cardano_meta.symbol; }
            if description.is_none() { description = cardano_meta.description; }
            if image_url.is_none() { image_url = cardano_meta.image_url; }
            if decimals.is_none() { decimals = cardano_meta.decimals; }
        }
    }

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

            // For beaming: always populate metadata (NFTs AND tokens)
            let use_metadata = is_nft || analyzed.is_beaming;

            Some((
                asset.app_id.clone(),
                analyzed.txid.clone(),
                0i32,
                height,
                asset.asset_type.clone(),
                supply,
                blockchain.to_string(),
                network.to_string(),
                if use_metadata { name.clone() } else { None },
                if use_metadata { symbol.clone() } else { None },
                if use_metadata { description.clone() } else { None },
                if use_metadata { image_url.clone() } else { None },
                if use_metadata { decimals } else { None },
                cardano_policy_id.clone(),
                cardano_asset_name.clone(),
                cardano_fingerprint.clone(),
            ))
        })
        .collect()
}

/// Fetch Cardano token metadata for a beaming transaction.
/// Parses the App from the asset's app_id, derives Cardano IDs, and fetches metadata.
async fn fetch_cardano_metadata_for_beaming(
    analyzed: &AnalyzedTx,
) -> Option<crate::infrastructure::cardano::metadata::CardanoTokenMetadata> {
    use crate::infrastructure::cardano::metadata;

    let app: charms_data::App = analyzed.app_id.parse().ok()?;
    metadata::fetch_metadata(&app).await
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
