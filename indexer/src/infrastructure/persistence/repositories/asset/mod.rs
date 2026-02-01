//! Asset repository module

pub mod helpers;
mod query;
pub mod save;
mod supply;

use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::domain::models::Asset;
use crate::infrastructure::persistence::error::DbError;

/// Repository for asset-related database operations
#[derive(Debug, Clone)]
pub struct AssetRepository {
    db: DatabaseConnection,
}

impl AssetRepository {
    /// Create a new AssetRepository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// [RJJ-SUPPLY] Save or update asset with correct supply logic
    pub async fn save_or_update_asset(&self, asset: &Asset, amount: i64) -> Result<(), DbError> {
        save::save_or_update_asset(&self.db, asset, amount).await
    }

    /// Save a single asset to the database (legacy method)
    pub async fn save_asset(&self, asset: &Asset) -> Result<(), DbError> {
        save::save_asset(&self.db, asset).await
    }

    /// Save multiple assets in a batch operation
    pub async fn save_batch(
        &self,
        assets: Vec<(
            String, // app_id
            String, // txid
            i32,    // vout_index
            String, // charm_id
            u64,    // block_height
            Value,  // data
            String, // asset_type
            String, // blockchain
            String, // network
        )>,
    ) -> Result<(), DbError> {
        save::save_batch(&self.db, assets).await
    }

    /// Find asset by app_id
    pub async fn find_by_app_id(&self, app_id: &str) -> Result<Option<Asset>, DbError> {
        query::find_by_app_id(&self.db, app_id).await
    }

    /// Get assets by charm_id
    pub async fn find_by_charm_id(&self, charm_id: &str) -> Result<Vec<Asset>, DbError> {
        query::find_by_charm_id(&self.db, charm_id).await
    }

    /// [RJJ-SUPPLY] Update supply when charms are marked as spent
    pub async fn update_supply_on_spent(
        &self,
        app_id: &str,
        amount: i64,
        asset_type: &str,
    ) -> Result<(), DbError> {
        supply::update_supply_on_spent(&self.db, app_id, amount, asset_type).await
    }

    /// Update NFT metadata (name, image_url) and mark as reference NFT
    pub async fn update_nft_metadata(
        &self,
        app_id: &str,
        name: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<(), DbError> {
        save::update_nft_metadata(&self.db, app_id, name, image_url).await
    }
}
