use std::collections::HashMap;
use std::sync::Arc;

use crate::db::repositories::asset_repository::AssetRepository;
use crate::entity::assets::Model as Asset;

/// Service for asset-related business logic
pub struct AssetService {
    asset_repository: Arc<AssetRepository>,
}

impl AssetService {
    /// Create a new asset service instance
    pub fn new(asset_repository: Arc<AssetRepository>) -> Self {
        Self { asset_repository }
    }

    /// Get assets with pagination and optional filtering
    pub async fn get_assets_paginated(
        &self,
        asset_type: Option<&str>,
        network: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<Asset>, u64), Box<dyn std::error::Error + Send + Sync>> {
        // Get filtered assets with pagination
        let assets = self
            .asset_repository
            .find_paginated(asset_type, network, limit, offset)
            .await?;

        // Get total count for pagination info
        let total = self
            .asset_repository
            .count_assets(asset_type, network)
            .await?;

        Ok((assets, total))
    }

    /// Get asset counts by type
    pub async fn get_asset_counts(&self) -> Result<HashMap<String, u64>, Box<dyn std::error::Error + Send + Sync>> {
        let mut counts = HashMap::new();

        // Get total count
        let total = self.asset_repository.count_assets(None, None).await?;
        counts.insert("total".to_string(), total);

        // Get counts by type
        let nft_count = self.asset_repository.count_assets(Some("nft"), None).await?;
        let token_count = self.asset_repository.count_assets(Some("token"), None).await?;
        let dapp_count = self.asset_repository.count_assets(Some("dapp"), None).await?;

        counts.insert("nft".to_string(), nft_count);
        counts.insert("token".to_string(), token_count);
        counts.insert("dapp".to_string(), dapp_count);

        Ok(counts)
    }

    /// Get asset by ID
    pub async fn get_asset_by_id(&self, id: i32) -> Result<Option<Asset>, Box<dyn std::error::Error + Send + Sync>> {
        self.asset_repository.find_by_id(id).await
    }

    /// Get asset by app_id
    pub async fn get_asset_by_app_id(&self, app_id: &str) -> Result<Option<Asset>, Box<dyn std::error::Error + Send + Sync>> {
        self.asset_repository.find_by_app_id(app_id).await
    }

    /// Get assets by type
    pub async fn get_assets_by_type(&self, asset_type: &str) -> Result<Vec<Asset>, Box<dyn std::error::Error + Send + Sync>> {
        self.asset_repository.find_by_asset_type(asset_type).await
    }

    /// Get assets by network
    pub async fn get_assets_by_network(&self, network: &str) -> Result<Vec<Asset>, Box<dyn std::error::Error + Send + Sync>> {
        self.asset_repository.find_by_network(network).await
    }
}
