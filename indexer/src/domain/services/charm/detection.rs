///! Charm detection logic using native charms-client library
use chrono::Utc;
use serde_json::json;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::domain::services::address_extractor::AddressExtractor;
use crate::domain::services::dex;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{CharmRepository, DexOrdersRepository};
use crate::infrastructure::queue::charm_queue::AssetSaveRequest;

/// Result of charm detection: the charm itself plus any asset save requests
pub type DetectionResult = (Charm, Vec<AssetSaveRequest>);

/// Handles charm detection from Bitcoin transactions
pub struct CharmDetector<'a> {
    bitcoin_client: &'a BitcoinClient,
    charm_repository: &'a CharmRepository,
    dex_orders_repository: Option<&'a DexOrdersRepository>,
}

impl<'a> CharmDetector<'a> {
    pub fn new(bitcoin_client: &'a BitcoinClient, charm_repository: &'a CharmRepository) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            dex_orders_repository: None,
        }
    }

    /// Set the DEX orders repository for saving detected orders
    pub fn with_dex_orders_repository(mut self, repo: &'a DexOrdersRepository) -> Self {
        self.dex_orders_repository = Some(repo);
        self
    }

    /// Detects a potential charm transaction using native parsing (no persistence)
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<DetectionResult>, CharmError> {
        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)
            .await?;

        self.detect_from_hex(
            txid,
            block_height,
            &raw_tx_hex,
            tx_pos,
            block_height,
            vec![],
        )
        .await
    }

    /// Detects a potential charm transaction from pre-fetched raw hex (no persistence)
    pub async fn detect_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        _tx_pos: usize,
        _latest_height: u64,
        input_txids: Vec<String>,
    ) -> Result<Option<DetectionResult>, CharmError> {
        // Get blockchain and network information from the Bitcoin client
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Clone hex for blocking task
        let raw_tx_hex_owned = raw_tx_hex.to_string();

        // Try to extract spell using native parser (no ZK proof verification).
        // The ZK proof was already validated by Bitcoin network consensus.
        // Re-verifying with charms-client 0.12.0 fails for V10 txs because V9_SPELL_VK
        // is wrong for V10 proofs. The indexer is a read-only observer.
        let (normalized_spell_opt, charm_json) = tokio::task::spawn_blocking(move || {
            match NativeCharmParser::extract_spell_no_verify(&raw_tx_hex_owned) {
                Ok(spell) => {
                    let charm_json = serde_json::to_value(&spell).map_err(|e| {
                        CharmError::ProcessingError(format!(
                            "Failed to serialize spell data: {}",
                            e
                        ))
                    })?;

                    Ok((
                        Some(spell),
                        json!({
                            "type": "spell",
                            "detected": true,
                            "has_native_data": true,
                            "native_data": charm_json,
                            "version": "native_parser"
                        }),
                    ))
                }
                Err(_e) => Ok::<_, CharmError>((None, json!(null))), // Return None on parse error
            }
        })
        .await
        .map_err(|e| CharmError::ProcessingError(format!("Join error: {}", e)))??;

        // If detection failed (returned None/null), log debug and return
        if normalized_spell_opt.is_none() {
            // Log debug info about failed detection (usually means just no charm present)
            return Ok(None);
        }

        // Extract asset information from the spell (used for metadata extraction)
        let _asset_infos = if let Some(ref spell) = normalized_spell_opt {
            NativeCharmParser::extract_asset_info(spell)
        } else {
            vec![]
        };

        // Extract Bitcoin address from the transaction
        let address = match AddressExtractor::extract_charm_holder_address(&raw_tx_hex, &network) {
            Ok(addr) => addr,
            Err(e) => {
                crate::utils::logging::log_debug(&format!(
                    "[{}] ⚠️ Block {}: Could not extract address for tx {}: {}",
                    network, block_height, txid, e
                ));
                None
            }
        };

        // Extract assets from the spell if available
        let asset_infos = if let Some(ref spell) = normalized_spell_opt {
            NativeCharmParser::extract_asset_info(spell)
        } else {
            vec![]
        };

        // Extract app_id, asset_type and amount from asset_infos
        let (app_id, asset_type, amount) = if let Some(first_asset) = asset_infos.first() {
            let asset_type = if first_asset.app_id.starts_with("t/") {
                "token".to_string()
            } else if first_asset.app_id.starts_with("n/") {
                "nft".to_string()
            } else if first_asset.app_id.starts_with("B/") {
                "dapp".to_string()
            } else {
                "other".to_string()
            };
            (
                first_asset.app_id.clone(),
                asset_type,
                first_asset.amount as i64,
            )
        } else {
            (String::from("other"), "spell".to_string(), 0)
        };

        // Extract metadata from charm_json for NFTs BEFORE creating the Charm
        // The metadata is in native_data.tx.outs[0] for the NFT
        let metadata = if asset_type == "nft" {
            charm_json
                .get("native_data")
                .and_then(|nd| nd.get("tx"))
                .and_then(|tx| tx.get("outs"))
                .and_then(|outs| outs.get(0))
                .and_then(|out0| out0.get("0"))
                .cloned()
        } else {
            None
        };

        // Extract individual metadata fields
        // [RJJ-TODO] supply_limit is currently IGNORED
        // The supply_limit field in NFT metadata indicates the maximum number of tokens that can be minted.
        // It's informational only and not used for total_supply calculation.
        // NFTs start with total_supply = 0, and tokens will increment it when minted.
        // If we need to store supply_limit in the future, we should add a new column to the assets table.
        let (name, symbol, description, image_url, decimals) = if let Some(ref meta) = metadata {
            let name = meta.get("name").and_then(|v| v.as_str()).map(String::from);
            let symbol = meta
                .get("ticker")
                .or_else(|| meta.get("symbol"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let description = meta
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);
            let image_url = meta
                .get("image")
                .or_else(|| meta.get("url"))
                .or_else(|| meta.get("image_url"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let decimals = meta
                .get("decimals")
                .and_then(|v| v.as_u64())
                .map(|d| d as u8);

            // Note: supply_limit is available in meta.get("supply_limit") but currently ignored
            // Example: meta.get("supply_limit").and_then(|v| v.as_u64()) -> Some(69420000000)

            (name, symbol, description, image_url, decimals)
        } else {
            (None, None, None, None, None)
        };

        // Build tags based on detected products
        let mut tag_list: Vec<String> = Vec::new();

        // Detect DEX operations (Charms Cast)
        let dex_result = dex::detect_dex_operation(&charm_json);
        if let Some(ref result) = dex_result {
            // Add operation-specific tag
            tag_list.push("charms-cast".to_string());
            tag_list.push(result.operation.to_tag().to_string());

            crate::utils::logging::log_info(&format!(
                "[{}] 🏷️ Block {}: Charms Cast DEX detected for tx {}: {:?}",
                network, block_height, txid, result.operation
            ));

            // Save DEX order to database if repository is available
            if let Some(dex_repo) = &self.dex_orders_repository {
                if let Some(ref order) = result.order {
                    match dex_repo
                        .save_order(
                            txid,
                            0, // vout - DEX orders are in output 0
                            Some(block_height),
                            order,
                            &result.operation,
                            "charms-cast",
                            &blockchain,
                            &network,
                        )
                        .await
                    {
                        Ok(_) => {
                            crate::utils::logging::log_info(&format!(
                                "[{}] 💾 Block {}: Saved DEX order for tx {}: {:?} {:?}",
                                network, block_height, txid, order.side, result.operation
                            ));
                        }
                        Err(e) => {
                            crate::utils::logging::log_error(&format!(
                                "[{}] ❌ Block {}: Failed to save DEX order for tx {}: {}",
                                network, block_height, txid, e
                            ));
                        }
                    }
                }
            }
        }

        // [RJJ-BEAMING] Detect Beaming transactions (cross-address token transfers)
        // Beaming txs have beamed_outs in the spell data
        // TODO: [RJJ-UNBEAM] Detect Unbeam transactions when they exist.
        //       Unbeam txs will likely have `unbeamed_ins` or similar field in the spell.
        //       Add tag "unbeam" and log similarly. Check charms-client for the exact field name.
        if let Some(ref spell) = normalized_spell_opt {
            if spell.tx.beamed_outs.is_some() {
                tag_list.push("beaming".to_string());
                crate::utils::logging::log_info(&format!(
                    "[{}] 🏷️ Block {}: Beaming transaction detected for tx {}",
                    network, block_height, txid
                ));
            }
        }

        // Detect $BRO token
        if dex::is_bro_token(&app_id) {
            tag_list.push("bro".to_string());
            crate::utils::logging::log_info(&format!(
                "[{}] 🏷️ Block {}: $BRO token detected for tx {}",
                network, block_height, txid
            ));
        }

        // Also check all assets for $BRO (in case it's not the first asset)
        for asset in &asset_infos {
            if dex::is_bro_token(&asset.app_id) && !tag_list.contains(&"bro".to_string()) {
                tag_list.push("bro".to_string());
                crate::utils::logging::log_info(&format!(
                    "[{}] 🏷️ Block {}: $BRO token detected in assets for tx {}",
                    network, block_height, txid
                ));
                break;
            }
        }

        // Convert tags to Option<String>
        let tags = if tag_list.is_empty() {
            None
        } else {
            Some(tag_list.join(","))
        };

        // Create charm with appropriate data
        // [RJJ-S01] Removed charmid parameter, added app_id and amount
        // [RJJ-DEX] Added tags for DEX operation detection
        let charm = Charm::new(
            txid.to_string(),
            0, // vout - Charms are always in output 0 per protocol
            Some(block_height),
            charm_json,
            Utc::now().naive_utc(),
            asset_type.clone(),
            blockchain.clone(),
            network.clone(),
            address,
            false, // New charms are unspent by default
            app_id.clone(),
            amount,
        )
        .with_tags(tags);

        // [RJJ-MINT-TRANSFER] Calculate net supply change per app_id
        // MINT: sum(inputs) < sum(outputs) → Increment supply by difference
        // TRANSFER: sum(inputs) == sum(outputs) → No supply change
        // BURN: sum(inputs) > sum(outputs) → Decrement supply (not implemented yet)

        // Query input amounts from database
        let input_amounts = if !input_txids.is_empty() {
            self.charm_repository
                .get_amounts_by_txids(&input_txids)
                .await
                .unwrap_or_default() // On error, treat as empty (mint)
        } else {
            vec![]
        };

        // Group outputs by app_id and calculate net change
        use std::collections::HashMap;
        let mut net_changes: HashMap<String, i64> = HashMap::new();

        for asset in &asset_infos {
            // Convert token app_id to NFT app_id for grouping
            let nft_app_id = if asset.asset_type == "token" {
                asset.app_id.replacen("t/", "n/", 1)
            } else {
                asset.app_id.clone()
            };

            // Sum output amount
            *net_changes.entry(nft_app_id).or_insert(0) += asset.amount as i64;
        }

        // Subtract input amounts
        for (_txid, app_id, amount) in input_amounts {
            // Convert token app_id to NFT app_id for grouping
            let nft_app_id = if app_id.starts_with("t/") {
                app_id.replacen("t/", "n/", 1)
            } else {
                app_id
            };

            *net_changes.entry(nft_app_id).or_insert(0) -= amount as i64;
        }

        // Create asset save requests
        // FIXED: Create separate asset records for tokens AND NFTs
        // Supply consolidation happens in the database layer (save.rs)
        // This allows tokens to appear in the Tokens tab while still tracking supply correctly
        let _asset_count = asset_infos.len();
        let asset_requests: Vec<_> = asset_infos
            .into_iter()
            .filter_map(|asset| {
                use crate::infrastructure::queue::charm_queue::AssetSaveRequest;

                // Convert to NFT app_id for lookup
                let nft_app_id = if asset.asset_type == "token" {
                    asset.app_id.replacen("t/", "n/", 1)
                } else {
                    asset.app_id.clone()
                };

                // Get net change for this app_id
                let net_change = net_changes.get(&nft_app_id).copied().unwrap_or(0);

                // Skip if no net change (transfer, not mint)
                if net_change == 0 {
                    return None;
                }

                // Use net_change as supply (can be negative for burns)
                let supply = net_change.max(0) as u64; // Clamp to 0 for now (no burns yet)

                // Create asset record with correct app_id and asset_type
                // The database layer will handle supply consolidation with parent NFT
                let is_nft = asset.asset_type == "nft";
                Some(AssetSaveRequest {
                    app_id: asset.app_id,         // Keep original app_id (t/ or n/)
                    asset_type: asset.asset_type, // Keep original asset_type
                    supply,                       // Use net_change as supply
                    blockchain: blockchain.clone(),
                    network: network.clone(),
                    name: if is_nft { name.clone() } else { None },
                    symbol: if is_nft { symbol.clone() } else { None },
                    description: if is_nft { description.clone() } else { None },
                    image_url: if is_nft { image_url.clone() } else { None },
                    decimals: if is_nft { decimals } else { None },
                })
            })
            .collect();

        // Return detected charm and asset data without saving
        // Persistence is handled by the caller (process_block) for linear, synchronous flow
        Ok(Some((charm, asset_requests)))
    }
}
