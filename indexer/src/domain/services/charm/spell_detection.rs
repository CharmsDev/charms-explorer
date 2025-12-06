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
use crate::domain::services::charm_queue_service::CharmQueueService;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{CharmRepository, SpellRepository};

/// [RJJ-S01] Handles spell-first charm detection from Bitcoin transactions
pub struct SpellDetector<'a> {
    bitcoin_client: &'a BitcoinClient,
    charm_repository: &'a CharmRepository,
    spell_repository: &'a SpellRepository,
    charm_queue_service: &'a Option<CharmQueueService>,
}

impl<'a> SpellDetector<'a> {
    pub fn new(
        bitcoin_client: &'a BitcoinClient,
        charm_repository: &'a CharmRepository,
        spell_repository: &'a SpellRepository,
        charm_queue_service: &'a Option<CharmQueueService>,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            spell_repository,
            charm_queue_service,
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
        latest_height: u64,
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

        // [RJJ-S01] STEP 1: Extract and verify spell using native parser
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

        crate::utils::logging::log_info(&format!(
            "[{}] 📜 Block {}: Spell saved for tx {}",
            network, block_height, txid
        ));

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
            } else {
                "other"
            };

            // Extract amount from spell data (native_data.tx.outs[index]["0"])
            let amount = if let Some(native_data) = spell_json.get("native_data") {
                if let Some(tx) = native_data.get("tx") {
                    if let Some(outs) = tx.get("outs").and_then(|v| v.as_array()) {
                        if let Some(out) = outs.get(index) {
                            out.get("0").and_then(|v| v.as_i64()).unwrap_or(0)
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };

            // TODO: Identify correct vout by finding outputs with 1000 or 330 satoshis
            // For now, use sequential vout (will be corrected in future update)
            let vout = (index + 1) as i32; // vout 1, 2, 3... (0 is spell)

            // Create charm-specific data (not the entire spell)
            let charm_data = json!({
                "app_id": asset_info.app_id,
                "data": null, // Charm-specific data (if available from spell)
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

            charms.push(charm.clone());

            // Save charm using queue if available
            if let Some(ref queue_service) = self.charm_queue_service {
                // TODO: [RJJ-ASSETS] Implementar lógica de consolidación de assets:
                // - Si existe NFT (n/HASH), NO crear asset para token (t/HASH)
                // - El supply de tokens debe asignarse al NFT
                // - Solo crear asset de token si NO existe NFT con el mismo hash
                // Por ahora, se crean assets individuales (se consolidarán en batch posterior)

                // TODO: [RJJ-SUPPLY] El supply debe calcularse desde UTXOs UNSPENT:
                // - Al crear nuevo charm: NO sumar al supply (aún no confirmado)
                // - Al marcar charm como spent: RESTAR del supply
                // - Al confirmar nuevo charm: SUMAR al supply
                // - En transferencias: supply se mantiene (gasta N, crea N)

                // TODO: [RJJ-BURN] Detectar operaciones de burn:
                // - Burn = gastar UTXO sin crear nueva (o crear con amount=0)
                // - Al detectar burn: RESTAR del supply
                // - Posible indicador: output a address OP_RETURN o address especial
                // - Necesita análisis de outputs de la transacción

                let asset_request = crate::infrastructure::queue::charm_queue::AssetSaveRequest {
                    app_id: asset_info.app_id.clone(),
                    asset_type: asset_type.to_string(),
                    supply: amount as u64,
                    blockchain: blockchain.clone(),
                    network: network.clone(),
                };

                if let Err(e) = queue_service
                    .save_charm_data(
                        &charm,
                        tx_pos as i64,
                        raw_tx_hex.to_string(),
                        latest_height,
                        vec![asset_request],
                    )
                    .await
                {
                    crate::utils::logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to queue charm (vout {}) for tx {}: {}",
                        network, block_height, vout, txid, e
                    ));
                }
            } else {
                // Direct save without queue
                if let Err(e) = self.charm_repository.save_charm(&charm).await {
                    crate::utils::logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to save charm (vout {}) for tx {}: {}",
                        network, block_height, vout, txid, e
                    ));
                }
            }
        }

        crate::utils::logging::log_info(&format!(
            "[{}] ✅ Block {}: Saved spell + {} charms for tx {}",
            network,
            block_height,
            charms.len(),
            txid
        ));

        Ok((Some(spell), charms))
    }
}
