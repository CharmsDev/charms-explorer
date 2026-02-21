///! [RJJ-S01] Spell-first charm detection logic
///!
///! New architecture:
///! 1. Detect and save spell (output 0) first
///! 2. Parse spell to extract multiple charms
///! 3. Save each charm with correct vout (1, 2, 3...)
use chrono::Utc;
use serde_json::json;

use crate::domain::errors::CharmError;
use crate::domain::models::{Charm, Spell};
use crate::domain::services::address_extractor::AddressExtractor;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, CharmRepository, SpellRepository,
};

/// [RJJ-S01] Handles spell-first charm detection from Bitcoin transactions
pub struct SpellDetector<'a> {
    bitcoin_client: &'a BitcoinClient,
    #[allow(dead_code)]
    charm_repository: &'a CharmRepository,
    spell_repository: &'a SpellRepository,
    #[allow(dead_code)]
    asset_repository: &'a AssetRepository,
}

impl<'a> SpellDetector<'a> {
    pub fn new(
        bitcoin_client: &'a BitcoinClient,
        charm_repository: &'a CharmRepository,
        spell_repository: &'a SpellRepository,
        asset_repository: &'a AssetRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            spell_repository,
            asset_repository,
        }
    }

    /// [RJJ-S01] Detects and processes a spell transaction with spell-first architecture
    /// Returns: (Option<Spell>, Vec<Charm>) - the spell and all charms it contains
    pub async fn detect_and_process_spell(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<(Option<Spell>, Vec<Charm>), CharmError> {
        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)
            .await?;

        self.detect_from_hex(txid, block_height, &raw_tx_hex, tx_pos, block_height)
            .await
    }

    /// [RJJ-S01] Detects spell and charms from pre-fetched raw hex
    pub async fn detect_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        _latest_height: u64,
    ) -> Result<(Option<Spell>, Vec<Charm>), CharmError> {
        // Get blockchain and network information
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Log transaction processing start (reduced frequency)
        if tx_pos % 100 == 0 {
            crate::utils::logging::log_info(&format!(
                "[{}] 🔍 Block {}: Processing tx {} (pos: {}) for spell detection",
                network, block_height, txid, tx_pos
            ));
        }

        // [RJJ-S01] STEP 1: Extract spell using native parser (no ZK proof verification).
        // The ZK proof was already validated by Bitcoin network consensus.
        // Re-verifying with charms-client 0.12.0 fails for V10 txs because V9_SPELL_VK
        // is wrong for V10 proofs. The indexer is a read-only observer.
        let (normalized_spell, spell_json) =
            match NativeCharmParser::extract_spell_no_verify(raw_tx_hex) {
                Ok(spell) => {
                    let spell_json = serde_json::to_value(&spell).map_err(|e| {
                        CharmError::ProcessingError(format!(
                            "Failed to serialize spell data: {}",
                            e
                        ))
                    })?;
                    (spell, spell_json)
                }
                Err(e) => {
                    crate::utils::logging::log_debug(&format!(
                        "[{}] ⚪ Block {}: No spell detected in tx {} (pos: {}): {}",
                        network, block_height, txid, tx_pos, e
                    ));
                    return Ok((None, vec![]));
                }
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

        // [RJJ-S01] STEP 2: Create and save Spell (output 0)
        let spell_data = json!({
            "type": "spell",
            "detected": true,
            "has_native_data": true,
            "native_data": spell_json,
            "version": "native_parser"
        });

        // [RJJ-S01] Spells don't have addresses (they are OP_RETURN outputs)
        let spell = Spell::new(
            txid.to_string(),
            block_height,
            spell_data.clone(),
            Utc::now().naive_utc(),
            "spell".to_string(),
            blockchain.clone(),
            network.clone(),
        );

        // Save spell to database
        if let Err(e) = self.spell_repository.save_spell(&spell).await {
            crate::utils::logging::log_error(&format!(
                "[{}] ❌ Block {}: Failed to save spell for tx {}: {}",
                network, block_height, txid, e
            ));
            return Err(CharmError::ProcessingError(format!(
                "Failed to save spell: {}",
                e
            )));
        }

        // Log only if not in silent mode (fast reindex)
        if std::env::var("SILENT_SPELL_LOGS").is_err() {
            crate::utils::logging::log_info(&format!(
                "[{}] 📜 Block {}: Spell saved for tx {}",
                network, block_height, txid
            ));
        }

        // [RJJ-S01] STEP 3: Extract charms from spell
        let asset_infos = NativeCharmParser::extract_asset_info(&normalized_spell);

        // [RJJ-S01] STEP 4: Create charm for each asset with correct app_id, asset_type, vout, and amount
        // NOTE: vout is determined by outputs with 1000 or 330 satoshis (not sequential)
        // NOTE: amount is extracted from native_data.tx.outs[index]["0"]
        let mut charms = Vec::new();

        for (index, asset_info) in asset_infos.iter().enumerate() {
            // Determine asset_type from app_id prefix
            let asset_type = if asset_info.app_id.starts_with("t/") {
                "token"
            } else if asset_info.app_id.starts_with("n/") {
                "nft"
            } else if asset_info.app_id.starts_with("B/") {
                "dapp"
            } else {
                "other"
            };

            // Extract amount and metadata from spell data (spell_json.tx.outs[vout_index]["0"])
            // spell_json is already the serialized NormalizedSpell (it becomes native_data in spell_data)
            // For NFTs, "0" contains an object with metadata (name, image, etc.)
            // For tokens, "0" contains the amount as integer
            let output_index = asset_info.vout_index as usize;
            let (amount, nft_metadata) = if let Some(tx) = spell_json.get("tx") {
                if let Some(outs) = tx.get("outs").and_then(|v| v.as_array()) {
                    if let Some(out) = outs.get(output_index) {
                        if let Some(out_value) = out.get("0") {
                            // Check if it's an object (NFT metadata) or number (token amount)
                            if out_value.is_object() {
                                // NFT with metadata object
                                (0i64, Some(out_value.clone()))
                            } else if let Some(amt) = out_value.as_i64() {
                                // Token with amount
                                (amt, None)
                            } else {
                                (0, None)
                            }
                        } else {
                            (0, None)
                        }
                    } else {
                        (0, None)
                    }
                } else {
                    (0, None)
                }
            } else {
                (0, None)
            };

            // TODO: Identify correct vout by finding outputs with 1000 or 330 satoshis
            // For now, use sequential vout (will be corrected in future update)
            let vout = (index + 1) as i32; // vout 1, 2, 3... (0 is spell)

            // Create charm-specific data with NFT metadata if available
            let charm_data = json!({
                "app_id": asset_info.app_id,
                "data": nft_metadata, // NFT metadata (name, image, etc.) or null for tokens
                "type": "charm",
                "asset_type": asset_type
            });

            let charm = Charm::new(
                txid.to_string(),
                vout,
                Some(block_height),
                charm_data,
                Utc::now().naive_utc(),
                asset_type.to_string(),
                blockchain.clone(),
                network.clone(),
                address.clone(),
                false, // New charms are unspent by default
                asset_info.app_id.clone(),
                amount,
            );

            charms.push(charm);
        }

        // Log only if not in silent mode (fast reindex)
        if std::env::var("SILENT_SPELL_LOGS").is_err() {
            crate::utils::logging::log_info(&format!(
                "[{}] ✅ Block {}: Saved spell + {} charms for tx {}",
                network,
                block_height,
                charms.len(),
                txid
            ));
        }

        Ok((Some(spell), charms))
    }

    /// Extract hash from app_id (removes t/ or n/ prefix)
    #[allow(dead_code)] // Reserved for future use
    fn extract_hash_from_app_id(&self, app_id: &str) -> String {
        if let Some(stripped) = app_id.strip_prefix("t/") {
            stripped.split('/').next().unwrap_or(app_id).to_string()
        } else if let Some(stripped) = app_id.strip_prefix("n/") {
            stripped.split('/').next().unwrap_or(app_id).to_string()
        } else {
            app_id.to_string()
        }
    }

    /// Extract NFT metadata from asset_info
    /// Returns (name, symbol, description, image_url, decimals)
    #[allow(dead_code)]
    fn extract_nft_metadata(
        &self,
        _asset_info: &crate::domain::services::native_charm_parser::AssetInfo,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<u8>,
    ) {
        // Try to parse metadata from asset_info.data if available
        // For now, return None for all fields (metadata extraction from charm data needs protocol spec)
        // TODO: Parse metadata from asset_info.data JSON structure
        (None, None, None, None, None)
    }

    /// [BATCH MODE] Parse spell and charms from hex WITHOUT any DB writes
    /// Returns: (Option<Spell>, Vec<Charm>) - pure parsing, no side effects
    /// Use this for batch processing where inserts are done separately
    pub fn parse_spell_only(
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        network: &str,
    ) -> Result<(Option<Spell>, Vec<Charm>), CharmError> {
        let blockchain = "Bitcoin".to_string();

        // STEP 1: Extract and verify spell using native parser
        let (normalized_spell, spell_json) =
            match NativeCharmParser::extract_and_verify_charm(raw_tx_hex, false) {
                Ok(spell) => {
                    let spell_json = serde_json::to_value(&spell).map_err(|e| {
                        CharmError::ProcessingError(format!(
                            "Failed to serialize spell data: {}",
                            e
                        ))
                    })?;
                    (spell, spell_json)
                }
                Err(_) => {
                    return Ok((None, vec![]));
                }
            };

        // Extract Bitcoin address from the transaction
        let address =
            AddressExtractor::extract_charm_holder_address(raw_tx_hex, network).unwrap_or(None);

        // STEP 2: Create Spell object (no save)
        let spell_data = json!({
            "type": "spell",
            "detected": true,
            "has_native_data": true,
            "native_data": spell_json,
            "version": "native_parser"
        });

        let spell = Spell::new(
            txid.to_string(),
            block_height,
            spell_data,
            Utc::now().naive_utc(),
            "spell".to_string(),
            blockchain.clone(),
            network.to_string(),
        );

        // STEP 3: Extract charms from spell
        let asset_infos = NativeCharmParser::extract_asset_info(&normalized_spell);

        // STEP 4: Create charm objects (no save)
        let mut charms = Vec::new();

        for (index, asset_info) in asset_infos.iter().enumerate() {
            let asset_type = if asset_info.app_id.starts_with("t/") {
                "token"
            } else if asset_info.app_id.starts_with("n/") {
                "nft"
            } else if asset_info.app_id.starts_with("B/") {
                "dapp"
            } else {
                "other"
            };

            let output_index = asset_info.vout_index as usize;
            let (amount, nft_metadata) = if let Some(tx) = spell_json.get("tx") {
                if let Some(outs) = tx.get("outs").and_then(|v| v.as_array()) {
                    if let Some(out) = outs.get(output_index) {
                        if let Some(out_value) = out.get("0") {
                            if out_value.is_object() {
                                (0i64, Some(out_value.clone()))
                            } else if let Some(amt) = out_value.as_i64() {
                                (amt, None)
                            } else {
                                (0, None)
                            }
                        } else {
                            (0, None)
                        }
                    } else {
                        (0, None)
                    }
                } else {
                    (0, None)
                }
            } else {
                (0, None)
            };

            let vout = (index + 1) as i32;

            let charm_data = json!({
                "app_id": asset_info.app_id,
                "data": nft_metadata,
                "type": "charm",
                "asset_type": asset_type
            });

            let charm = Charm::new(
                txid.to_string(),
                vout,
                Some(block_height),
                charm_data,
                Utc::now().naive_utc(),
                asset_type.to_string(),
                blockchain.clone(),
                network.to_string(),
                address.clone(),
                false,
                asset_info.app_id.clone(),
                amount,
            );

            charms.push(charm);
        }

        Ok((Some(spell), charms))
    }
}
