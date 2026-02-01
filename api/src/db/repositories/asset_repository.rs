use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::sync::Arc;

use crate::entity::assets::{Column, Entity as Asset, Model};

/// Repository for asset database operations
#[derive(Clone)]
pub struct AssetRepository {
    db: Arc<DatabaseConnection>,
}

impl AssetRepository {
    /// Create a new asset repository instance
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find assets with pagination and optional filtering
    /// Note: Reference NFTs (is_reference_nft = true) are excluded from NFT listings
    pub async fn find_paginated(
        &self,
        asset_type: Option<&str>,
        network: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = Asset::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(Column::AssetType.eq(asset_type));

            // For NFT listings, exclude reference NFTs (they are hidden, only used for token metadata)
            if asset_type == "nft" {
                query = query.filter(Column::IsReferenceNft.eq(false));
            }
        }

        if let Some(network) = network {
            query = query.filter(Column::Network.eq(network));
        }

        let assets = query
            .order_by_desc(Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await?;

        Ok(assets)
    }

    /// Count assets with optional filtering
    /// Note: Reference NFTs are excluded from NFT counts
    pub async fn count_assets(
        &self,
        asset_type: Option<&str>,
        network: Option<&str>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = Asset::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(Column::AssetType.eq(asset_type));

            // For NFT counts, exclude reference NFTs
            if asset_type == "nft" {
                query = query.filter(Column::IsReferenceNft.eq(false));
            }
        }

        if let Some(network) = network {
            query = query.filter(Column::Network.eq(network));
        }

        let count = query.count(self.db.as_ref()).await?;
        Ok(count)
    }

    /// Find asset by ID
    pub async fn find_by_id(
        &self,
        id: i32,
    ) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let asset = Asset::find_by_id(id).one(self.db.as_ref()).await?;
        Ok(asset)
    }

    /// Find asset by app_id
    pub async fn find_by_app_id(
        &self,
        app_id: &str,
    ) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let asset = Asset::find()
            .filter(Column::AppId.eq(app_id))
            .one(self.db.as_ref())
            .await?;
        Ok(asset)
    }

    /// Find assets by multiple app_ids
    pub async fn find_by_app_ids(
        &self,
        app_ids: Vec<String>,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let assets = Asset::find()
            .filter(Column::AppId.is_in(app_ids))
            .all(self.db.as_ref())
            .await?;
        Ok(assets)
    }

    /// Find assets by asset type
    #[allow(dead_code)] // Available via AssetService
    pub async fn find_by_asset_type(
        &self,
        asset_type: &str,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let assets = Asset::find()
            .filter(Column::AssetType.eq(asset_type))
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        Ok(assets)
    }

    /// Find assets by network
    #[allow(dead_code)] // Available via AssetService
    pub async fn find_by_network(
        &self,
        network: &str,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let assets = Asset::find()
            .filter(Column::Network.eq(network))
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        Ok(assets)
    }

    /// Get all assets
    #[allow(dead_code)] // Reserved for future use
    pub async fn find_all(&self) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let assets = Asset::find()
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        Ok(assets)
    }

    /// Count assets by asset type
    #[allow(dead_code)] // Reserved for future use
    pub async fn count_by_type(
        &self,
        asset_type: &str,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let count = Asset::find()
            .filter(Column::AssetType.eq(asset_type))
            .count(self.db.as_ref())
            .await?;
        Ok(count as i64)
    }

    /// Find reference NFT by hash (for token metadata lookup)
    /// Searches for NFTs where app_id starts with "n/{hash}/"
    /// Prioritizes NFTs with metadata (name, image_url) over empty ones
    pub async fn find_reference_nft_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let pattern = format!("n/{}/%", hash);

        // Get all NFTs matching the hash pattern
        let assets = Asset::find()
            .filter(Column::AssetType.eq("nft"))
            .filter(Column::AppId.like(&pattern))
            .order_by_asc(Column::BlockHeight)
            .all(self.db.as_ref())
            .await?;

        if assets.is_empty() {
            return Ok(None);
        }

        // Prioritize NFT with metadata (name or image_url present)
        // This handles cases where multiple NFTs exist for the same hash (different vouts)
        for asset in &assets {
            if asset.name.is_some() || asset.image_url.is_some() {
                return Ok(Some(asset.clone()));
            }
        }

        // Fallback to first NFT if none have metadata
        Ok(Some(assets.into_iter().next().unwrap()))
    }

    /// Get max total_supply from all assets matching a base app_id prefix
    /// Used to get the correct total supply for tokens with multiple outputs (:0, :1, etc.)
    pub async fn get_max_total_supply_by_prefix(
        &self,
        base_app_id: &str,
    ) -> Result<Option<rust_decimal::Decimal>, Box<dyn std::error::Error + Send + Sync>> {
        let pattern = format!("{}:%", base_app_id);

        let assets = Asset::find()
            .filter(Column::AppId.like(&pattern))
            .all(self.db.as_ref())
            .await?;

        // Find max total_supply among all matching assets
        let max_supply = assets.into_iter().filter_map(|a| a.total_supply).max();

        Ok(max_supply)
    }
}
