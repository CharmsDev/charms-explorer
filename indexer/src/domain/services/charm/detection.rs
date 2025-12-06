///! Charm detection logic using native charms-client library
use chrono::Utc;
use serde_json::json;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::domain::services::address_extractor::AddressExtractor;
use crate::domain::services::charm_queue_service::CharmQueueService;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::CharmRepository;

/// Handles charm detection from Bitcoin transactions
pub struct CharmDetector<'a> {
    bitcoin_client: &'a BitcoinClient,
    charm_repository: &'a CharmRepository,
    charm_queue_service: &'a Option<CharmQueueService>,
}

impl<'a> CharmDetector<'a> {
    pub fn new(
        bitcoin_client: &'a BitcoinClient,
        charm_repository: &'a CharmRepository,
        charm_queue_service: &'a Option<CharmQueueService>,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            charm_queue_service,
        }
    }

    /// Detects and processes a potential charm transaction using native parsing
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
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

    /// Detects and processes a potential charm transaction from pre-fetched raw hex
    pub async fn detect_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        latest_height: u64,
        input_txids: Vec<String>,
    ) -> Result<Option<Charm>, CharmError> {
        // Get blockchain and network information from the Bitcoin client
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Clone hex for blocking task
        let raw_tx_hex_owned = raw_tx_hex.to_string();

        // Try to extract and verify spell using native parser in a blocking task
        // to avoid blocking the async runtime with CPU-intensive work
        let (normalized_spell_opt, charm_json) = tokio::task::spawn_blocking(move || {
            match NativeCharmParser::extract_and_verify_charm(&raw_tx_hex_owned, false) {
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
                Err(e) => Ok::<_, CharmError>((None, json!(null))), // Return None on parse error, log outside if needed
            }
        })
        .await
        .map_err(|e| CharmError::ProcessingError(format!("Join error: {}", e)))??;

        // If detection failed (returned None/null), log debug and return
        if normalized_spell_opt.is_none() {
            // Log debug info about failed detection (usually means just no charm present)
            return Ok(None);
        }

        // Extract asset information from the spell
        let asset_infos = if let Some(ref spell) = normalized_spell_opt {
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
                .get("url")
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

        // Create charm with appropriate data
        // [RJJ-S01] Removed charmid parameter, added app_id and amount
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
        );

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
        for (txid, app_id, amount) in input_amounts {
            // Convert token app_id to NFT app_id for grouping
            let nft_app_id = if app_id.starts_with("t/") {
                app_id.replacen("t/", "n/", 1)
            } else {
                app_id
            };

            *net_changes.entry(nft_app_id).or_insert(0) -= amount as i64;
        }

        // Create asset save requests
        // IMPORTANT: Only NFTs create assets. Tokens increment the NFT's supply.
        // Tokens with the same hash as an NFT (t/HASH vs n/HASH) do NOT create separate assets.
        let asset_count = asset_infos.len();
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

                // Only create asset for NFTs
                // Tokens will increment the NFT's supply via ON CONFLICT DO UPDATE in the database
                if asset.asset_type == "nft" {
                    Some(AssetSaveRequest {
                        app_id: asset.app_id,
                        asset_type: asset.asset_type,
                        supply: 0, // NFTs start with supply = 0
                        blockchain: blockchain.clone(),
                        network: network.clone(),
                        name: name.clone(),
                        symbol: symbol.clone(),
                        description: description.clone(),
                        image_url: image_url.clone(),
                        decimals,
                    })
                } else if asset.asset_type == "token" {
                    // Tokens increment the NFT's supply by net_change (not full amount)
                    Some(AssetSaveRequest {
                        app_id: nft_app_id.clone(),    // Use NFT app_id
                        asset_type: "nft".to_string(), // Reference the NFT
                        supply,                        // Use net_change as supply
                        blockchain: blockchain.clone(),
                        network: network.clone(),
                        name: None, // Don't override NFT metadata
                        symbol: None,
                        description: None,
                        image_url: None,
                        decimals: None,
                    })
                } else {
                    // Other types (if any) create their own assets
                    Some(AssetSaveRequest {
                        app_id: asset.app_id,
                        asset_type: asset.asset_type,
                        supply, // Use net_change as supply
                        blockchain: blockchain.clone(),
                        network: network.clone(),
                        name: None,
                        symbol: None,
                        description: None,
                        image_url: None,
                        decimals: None,
                    })
                }
            })
            .collect();

        // Save the charm to the database (using queue if available, otherwise direct save)
        if let Some(ref queue_service) = self.charm_queue_service {
            match queue_service
                .save_charm_data(
                    &charm,
                    tx_pos as i64,
                    raw_tx_hex.to_string(),
                    latest_height,
                    asset_requests,
                )
                .await
            {
                Ok(_) => Ok(Some(charm)),
                Err(e) => {
                    crate::utils::logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to queue charm data for tx {} at position {}: {}",
                        network, block_height, txid, tx_pos, e
                    ));
                    Err(CharmError::ProcessingError(format!(
                        "Failed to queue charm data: {}",
                        e
                    )))
                }
            }
        } else {
            crate::utils::logging::log_info(&format!(
                "[{}] 💾 Block {}: Saving charm for tx {} to database (direct)",
                network, block_height, txid
            ));

            match self.charm_repository.save_charm(&charm).await {
                Ok(_) => {
                    crate::utils::logging::log_info(&format!(
                        "[{}] ✅ Block {}: Successfully saved charm for tx {} with {} assets",
                        network, block_height, txid, asset_count
                    ));
                    Ok(Some(charm))
                }
                Err(e) => {
                    crate::utils::logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to save native charm for tx {} at position {}: {}",
                        network, block_height, txid, tx_pos, e
                    ));
                    Err(CharmError::ProcessingError(format!(
                        "Failed to save native charm: {}",
                        e
                    )))
                }
            }
        }
    }
}
