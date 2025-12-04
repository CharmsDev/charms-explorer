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
    pub async fn count_assets(
        &self,
        asset_type: Option<&str>,
        network: Option<&str>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = Asset::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(Column::AssetType.eq(asset_type));
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
    pub async fn find_all(&self) -> Result<Vec<Model>, Box<dyn std::error::Error + Send + Sync>> {
        let assets = Asset::find()
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        Ok(assets)
    }
}
