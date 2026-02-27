///! Charm service - persistence and spent-tracking for charms.
///!
///! Detection is handled by `domain::services::tx_analyzer` (shared by all code paths).
///! This module provides:
///! - `persistence`: Batch save operations for charms and assets
///! - `spent_tracking`: Marking charms as spent when UTXOs are consumed
mod persistence;
mod spent_tracking;

use std::fmt;

use crate::domain::errors::CharmError;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, CharmRepository, DexOrdersRepository, SpellRepository, StatsHoldersRepository,
};
use persistence::CharmPersistence;
use spent_tracking::SpentTracker;

/// Main service for charm persistence, spent tracking, and repository access.
/// Detection is handled by `tx_analyzer::analyze_tx`.
#[derive(Clone)]
pub struct CharmService {
    charm_repository: CharmRepository,
    asset_repository: AssetRepository,
    spell_repository: SpellRepository,
    stats_holders_repository: StatsHoldersRepository,
    dex_orders_repository: DexOrdersRepository,
}

impl fmt::Debug for CharmService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CharmService").finish_non_exhaustive()
    }
}

impl CharmService {
    /// Creates a new CharmService with required dependencies
    pub fn new(
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
        spell_repository: SpellRepository,
        stats_holders_repository: StatsHoldersRepository,
        dex_orders_repository: DexOrdersRepository,
    ) -> Self {
        Self {
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

    /// Get reference to charm repository (for supply calculations in block processor)
    pub fn get_charm_repository(&self) -> &CharmRepository {
        &self.charm_repository
    }

    /// Get reference to dex_orders repository (for saving DEX orders in block processor)
    pub fn get_dex_orders_repository(&self) -> &DexOrdersRepository {
        &self.dex_orders_repository
    }

    /// Get list of block heights that already have charms processed
    pub async fn get_processed_block_heights(&self, network: &str) -> Result<Vec<u64>, CharmError> {
        self.charm_repository
            .get_distinct_block_heights(network)
            .await
            .map_err(|e| CharmError::DetectionError(e.to_string()))
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
        // For tokens (t/): use -amount as balance delta
        // For NFTs (n/): use -1 as balance delta (NFT ownership count, not the raw amount)
        if !charm_info.is_empty() {
            let holder_updates: Vec<(String, String, i64, i32)> = charm_info
                .into_iter()
                .filter_map(|(app_id, address, amount)| {
                    if app_id.starts_with("t/") {
                        // Token: convert t/ to n/ for consolidation, use actual negative amount
                        let nft_app_id = app_id.replacen("t/", "n/", 1);
                        Some((nft_app_id, address, -amount, 0))
                    } else if app_id.starts_with("n/") {
                        // NFT: keep n/ app_id, use -1 (ownership count)
                        Some((app_id, address, -1_i64, 0))
                    } else {
                        None
                    }
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
