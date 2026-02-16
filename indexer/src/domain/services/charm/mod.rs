///! Charm service - Main entry point for charm-related operations
///!
///! This module provides a unified interface for charm detection, persistence, and tracking.
///! It delegates to specialized sub-modules for better code organization:
///!
///! - `detection`: Charm detection from Bitcoin transactions
///! - `persistence`: Batch save operations for charms and assets
///! - `spent_tracking`: Marking charms as spent when UTXOs are consumed
///!
///! [RJJ-S01] Updated to support spell-first architecture:
///! 1. Save spell (output 0) first
///! 2. Parse spell to extract multiple charms
///! 3. Save each charm with correct vout (1, 2, 3...)
mod detection;
mod persistence;
pub mod spell_detection; // [BATCH MODE] Made public for direct parsing access
mod spent_tracking; // [RJJ-S01] New spell-first detection

use std::fmt;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, CharmRepository, DexOrdersRepository, SpellRepository, StatsHoldersRepository,
};
use detection::CharmDetector;
pub use detection::DetectionResult;
use persistence::CharmPersistence;
use spell_detection::SpellDetector;
use spent_tracking::SpentTracker;

/// Main service for charm detection, processing and storage
/// [RJJ-S01] Now includes SpellRepository for spell-first architecture
/// [RJJ-STATS-HOLDERS] Now includes StatsHoldersRepository for holder statistics
/// [RJJ-DEX] Now includes DexOrdersRepository for Cast DEX order tracking
#[derive(Clone)]
pub struct CharmService {
    bitcoin_client: BitcoinClient,
    charm_repository: CharmRepository,
    asset_repository: AssetRepository,
    spell_repository: SpellRepository,
    stats_holders_repository: StatsHoldersRepository,
    dex_orders_repository: DexOrdersRepository,
}

impl fmt::Debug for CharmService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CharmService")
            .field("bitcoin_client", &self.bitcoin_client)
            .finish_non_exhaustive()
    }
}

impl CharmService {
    /// Creates a new CharmService with required dependencies
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
        spell_repository: SpellRepository,
        stats_holders_repository: StatsHoldersRepository,
        dex_orders_repository: DexOrdersRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            asset_repository,
            spell_repository,
            stats_holders_repository,
            dex_orders_repository,
        }
    }

    // ==================== Repository Access Methods ====================

    /// [RJJ-STATS-HOLDERS] Get reference to stats_holders repository
    pub fn get_stats_holders_repository(&self) -> &StatsHoldersRepository {
        &self.stats_holders_repository
    }

    /// Get list of block heights that already have charms processed
    pub async fn get_processed_block_heights(&self, network: &str) -> Result<Vec<u64>, CharmError> {
        self.charm_repository
            .get_distinct_block_heights(network)
            .await
            .map_err(|e| CharmError::DetectionError(e.to_string()))
    }

    // ==================== Detection Methods ====================
    // Detection methods return DetectionResult = (Charm, Vec<AssetSaveRequest>)
    // They do NOT persist anything — the caller handles all persistence.

    /// Detect a charm from a transaction (fetches raw tx from node)
    pub async fn detect_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
    ) -> Result<Option<DetectionResult>, CharmError> {
        let detector = CharmDetector::new(&self.bitcoin_client, &self.charm_repository)
            .with_dex_orders_repository(&self.dex_orders_repository);
        detector
            .detect_and_process_charm(txid, block_height, block_hash, 0)
            .await
    }

    /// Detect a charm from pre-fetched raw hex
    pub async fn detect_charm_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
    ) -> Result<Option<DetectionResult>, CharmError> {
        self.detect_charm_from_hex_with_context(
            txid,
            block_height,
            raw_tx_hex,
            tx_pos,
            block_height,
            vec![],
        )
        .await
    }

    /// Detect a charm from pre-fetched raw hex with latest height and input txids
    pub async fn detect_charm_from_hex_with_context(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        latest_height: u64,
        input_txids: Vec<String>,
    ) -> Result<Option<DetectionResult>, CharmError> {
        let detector = CharmDetector::new(&self.bitcoin_client, &self.charm_repository)
            .with_dex_orders_repository(&self.dex_orders_repository);
        detector
            .detect_from_hex(
                txid,
                block_height,
                raw_tx_hex,
                tx_pos,
                latest_height,
                input_txids,
            )
            .await
    }

    // ==================== Legacy Detection Methods (backward compat) ====================

    /// Legacy: detect and process charm (returns only Charm, no assets)
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
    ) -> Result<Option<Charm>, CharmError> {
        Ok(self
            .detect_charm(txid, block_height, block_hash)
            .await?
            .map(|(charm, _)| charm))
    }

    /// Legacy: detect from hex (returns only Charm, no assets)
    pub async fn detect_and_process_charm_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
        Ok(self
            .detect_charm_from_hex(txid, block_height, raw_tx_hex, tx_pos)
            .await?
            .map(|(charm, _)| charm))
    }

    /// Legacy: detect from hex with latest height (returns only Charm, no assets)
    pub async fn detect_and_process_charm_from_hex_with_latest(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        latest_height: u64,
        input_txids: Vec<String>,
    ) -> Result<Option<Charm>, CharmError> {
        Ok(self
            .detect_charm_from_hex_with_context(
                txid,
                block_height,
                raw_tx_hex,
                tx_pos,
                latest_height,
                input_txids,
            )
            .await?
            .map(|(charm, _)| charm))
    }

    // ==================== Spell-First Detection Methods ====================

    /// Detects a spell transaction using spell-first architecture
    pub async fn detect_and_process_spell(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<(Option<crate::domain::models::Spell>, Vec<Charm>), CharmError> {
        let detector = SpellDetector::new(
            &self.bitcoin_client,
            &self.charm_repository,
            &self.spell_repository,
            &self.asset_repository,
        );
        detector
            .detect_and_process_spell(txid, block_height, block_hash, tx_pos)
            .await
    }

    /// Detects a spell from pre-fetched raw hex
    pub async fn detect_and_process_spell_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        latest_height: u64,
    ) -> Result<(Option<crate::domain::models::Spell>, Vec<Charm>), CharmError> {
        let detector = SpellDetector::new(
            &self.bitcoin_client,
            &self.charm_repository,
            &self.spell_repository,
            &self.asset_repository,
        );
        detector
            .detect_from_hex(txid, block_height, raw_tx_hex, tx_pos, latest_height)
            .await
    }

    // ==================== Persistence Methods ====================

    /// Optimize DB session for high-throughput writer tasks
    pub async fn optimize_writer_session(&self) -> Result<(), CharmError> {
        let persistence = CharmPersistence::new(&self.charm_repository, &self.asset_repository);
        persistence.optimize_writer_session().await
    }

    /// Saves multiple charms in a single database operation
    /// [RJJ-S01] Updated: replaced charmid with vout, added app_id and amount
    /// [RJJ-ADDRESS] Added address field
    /// [RJJ-DEX] Added tags field
    pub async fn save_batch(
        &self,
        charms: Vec<(
            String,            // txid
            i32,               // vout
            u64,               // block_height
            serde_json::Value, // data
            String,            // asset_type
            String,            // blockchain
            String,            // network
            Option<String>,    // address
            String,            // app_id
            i64,               // amount
            Option<String>,    // tags
        )>,
    ) -> Result<(), CharmError> {
        let persistence = CharmPersistence::new(&self.charm_repository, &self.asset_repository);
        persistence.save_charm_batch(charms).await
    }

    /// Save a batch of transactions to the repository
    pub async fn save_transaction_batch(
        &self,
        batch: Vec<(
            String,
            u64,
            i64,
            serde_json::Value,
            serde_json::Value,
            i32,
            bool,
            String,
            String,
        )>,
    ) -> Result<(), CharmError> {
        let persistence = CharmPersistence::new(&self.charm_repository, &self.asset_repository);
        persistence.save_transaction_batch(batch).await
    }

    /// Save a batch of assets to the repository
    pub async fn save_asset_batch(
        &self,
        batch: Vec<(
            String,         // app_id
            String,         // txid
            i32,            // vout
            u64,            // block_height
            String,         // asset_type
            u64,            // supply
            String,         // blockchain
            String,         // network
            Option<String>, // name
            Option<String>, // symbol
            Option<String>, // description
            Option<String>, // image_url
            Option<u8>,     // decimals
        )>,
    ) -> Result<(), CharmError> {
        let persistence = CharmPersistence::new(&self.charm_repository, &self.asset_repository);
        persistence.save_asset_batch(batch).await
    }

    // ==================== Spent Tracking Methods ====================

    /// Get unspent charms by (txid, vout) pairs - used to check actual DB state
    pub async fn get_unspent_charms_by_txid_vout(
        &self,
        txid_vouts: Vec<(String, i32)>,
    ) -> Result<Vec<(String, i32, String, Option<String>, i64)>, CharmError> {
        self.charm_repository
            .get_unspent_charms_by_txid_vout(txid_vouts)
            .await
            .map_err(|e| {
                CharmError::ProcessingError(format!("Failed to get unspent charms: {}", e))
            })
    }

    /// Mark a charm as spent by its txid and vout
    /// [RJJ-S01] Updated: now requires both txid and vout
    pub async fn mark_charm_as_spent(&self, txid: &str, vout: i32) -> Result<(), CharmError> {
        let tracker = SpentTracker::new(&self.charm_repository);
        tracker.mark_charm_as_spent(txid, vout).await
    }

    /// Mark multiple charms as spent in a batch using (txid, vout) pairs
    /// Also updates asset supply and stats_holders with negative amounts
    pub async fn mark_charms_as_spent_batch(
        &self,
        txid_vouts: Vec<(String, i32)>,
    ) -> Result<(), CharmError> {
        // 1. Get charm info before marking as spent (for stats_holders update)
        let charm_info = self
            .charm_repository
            .get_charms_for_spent_update(txid_vouts.clone())
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to get charm info: {}", e)))?;

        // 2. Mark charms as spent
        let tracker = SpentTracker::new(&self.charm_repository);
        tracker.mark_charms_as_spent_batch(txid_vouts).await?;

        // 2.5. Update asset supply for spent charms
        for (app_id, _address, amount) in &charm_info {
            // Determine asset_type from app_id prefix
            let asset_type = if app_id.starts_with("t/") {
                "token"
            } else if app_id.starts_with("n/") {
                "nft"
            } else {
                "other"
            };

            if let Err(e) = self
                .asset_repository
                .update_supply_on_spent(app_id, *amount, asset_type)
                .await
            {
                crate::utils::logging::log_warning(&format!(
                    "[CharmService] Failed to update supply for {}: {}",
                    app_id, e
                ));
            }
        }

        // 3. Update stats_holders with negative amounts (reduce balances)
        // [RJJ-TOKEN-METADATA] Convert token app_ids to NFT app_ids for consolidation
        if !charm_info.is_empty() {
            let holder_updates: Vec<(String, String, i64, i32)> = charm_info
                .into_iter()
                .map(|(app_id, address, amount)| {
                    // Convert token app_id (t/HASH) to NFT app_id (n/HASH) for consolidation
                    let nft_app_id = if app_id.starts_with("t/") {
                        app_id.replacen("t/", "n/", 1)
                    } else {
                        app_id
                    };
                    (nft_app_id, address, -amount, 0) // Negative amount, block_height not important for spent
                })
                .collect();

            if let Err(e) = self
                .stats_holders_repository
                .update_holders_batch(holder_updates)
                .await
            {
                crate::utils::logging::log_warning(&format!(
                    "[CharmService] Failed to update stats_holders for spent charms: {}",
                    e
                ));
            }
        }

        Ok(())
    }
}
