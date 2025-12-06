//! Asset repository for database operations

use chrono::{DateTime, FixedOffset, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set};
use serde_json::Value;

use crate::domain::models::Asset;
use crate::infrastructure::persistence::entities::{assets, prelude::*};
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

    /// Save or update asset with supply accumulation
    pub async fn save_or_update_asset(&self, asset: &Asset, amount: i64) -> Result<(), DbError> {
        // Processing asset silently

        // Check if asset already exists
        let existing_asset = Assets::find()
            .filter(assets::Column::AppId.eq(&asset.app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        match existing_asset {
            Some(existing) => {
                // Asset exists, update supply using Decimal for large numbers
                let old_supply = existing.total_supply.unwrap_or(Decimal::ZERO);
                let amount_decimal = Decimal::from(amount);
                let new_supply = old_supply + amount_decimal;
                // Reduced logging for asset updates

                let update_model = assets::ActiveModel {
                    id: Set(existing.id),
                    total_supply: Set(Some(new_supply)),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };

                Assets::update(update_model)
                    .exec(&self.db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;

                // Asset updated silently
            }
            None => {
                // Asset doesn't exist, create new one
                // Creating new asset silently

                let active_model = assets::ActiveModel {
                    id: NotSet,
                    app_id: Set(asset.app_id.clone()),
                    txid: Set(asset.txid.clone()),
                    vout_index: Set(asset.vout_index),
                    charm_id: Set(asset.charm_id.clone()),
                    block_height: Set(asset.block_height as i32),
                    date_created: Set(DateTime::<FixedOffset>::from(
                        DateTime::<Utc>::from_naive_utc_and_offset(asset.date_created, Utc),
                    )),
                    data: Set(asset.data.clone()),
                    asset_type: Set(asset.asset_type.clone()),
                    blockchain: Set(asset.blockchain.clone()),
                    network: Set(asset.network.clone()),
                    name: Set(None),
                    symbol: Set(None),
                    description: Set(None),
                    image_url: Set(None),
                    total_supply: Set(Some(Decimal::from(amount))),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                };

                Assets::insert(active_model)
                    .exec(&self.db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;

                // Asset created silently
            }
        }

        Ok(())
    }

    /// Save a single asset to the database (legacy method)
    pub async fn save_asset(&self, asset: &Asset) -> Result<(), DbError> {
        // Use the new method with amount = 1 for backward compatibility
        self.save_or_update_asset(asset, 1).await
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
        if assets.is_empty() {
            return Ok(());
        }

        let assets_count = assets.len();

        // Log only for larger batches to reduce noise
        if assets_count > 5 {
            println!("💾 Batch saving {} assets to database", assets_count);
        }

        let now = Utc::now();
        let active_models: Vec<assets::ActiveModel> = assets
            .into_iter()
            .map(
                |(
                    app_id,
                    txid,
                    vout_index,
                    charm_id,
                    block_height,
                    data,
                    asset_type,
                    blockchain,
                    network,
                )| {
                    // Extract supply from data JSON
                    let supply = data.get("supply").and_then(|v| v.as_u64()).unwrap_or(1);

                    // Extract metadata from data JSON
                    let name = data.get("name").and_then(|v| v.as_str()).map(String::from);
                    let symbol = data
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let description = data
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let image_url = data
                        .get("image_url")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    // Note: decimals is stored in data JSON, not as separate column

                    assets::ActiveModel {
                        id: NotSet,
                        app_id: Set(app_id),
                        txid: Set(txid),
                        vout_index: Set(vout_index),
                        charm_id: Set(charm_id),
                        block_height: Set(block_height as i32),
                        date_created: Set(now.into()),
                        data: Set(data),
                        asset_type: Set(asset_type),
                        blockchain: Set(blockchain),
                        network: Set(network),
                        name: Set(name),
                        symbol: Set(symbol),
                        description: Set(description),
                        image_url: Set(image_url),
                        total_supply: Set(Some(Decimal::from(supply))),
                        created_at: Set(now.into()),
                        updated_at: Set(now.into()),
                    }
                },
            )
            .collect();

        // Insert or update assets using ON CONFLICT DO UPDATE
        // When a token arrives with the same app_id as an NFT, increment the supply
        for model in active_models {
            let app_id = model.app_id.clone().unwrap();
            let supply_to_add = model
                .total_supply
                .clone()
                .unwrap()
                .unwrap_or(Decimal::from(0));

            // Try to find existing asset
            let existing = Assets::find()
                .filter(assets::Column::AppId.eq(&app_id))
                .one(&self.db)
                .await
                .map_err(DbError::SeaOrmError)?;

            if let Some(existing_asset) = existing {
                // Asset exists - increment supply (for tokens adding to NFT)
                let current_supply = existing_asset.total_supply.unwrap_or(Decimal::from(0));
                let new_supply = current_supply + supply_to_add;

                let mut update_model: assets::ActiveModel = existing_asset.into();
                update_model.total_supply = Set(Some(new_supply));
                update_model.updated_at = Set(now.into());

                // Only update metadata if provided (NFT creation, not token increment)
                if model.name.clone().unwrap().is_some() {
                    update_model.name = model.name;
                    update_model.symbol = model.symbol;
                    update_model.description = model.description;
                    update_model.image_url = model.image_url;
                }

                update_model
                    .update(&self.db)
                    .await
                    .map_err(DbError::SeaOrmError)?;
            } else {
                // Asset doesn't exist - insert new
                model.insert(&self.db).await.map_err(DbError::SeaOrmError)?;
            }
        }

        // Only log for larger batches
        if assets_count > 5 {
            println!("✅ Batch saved {} assets successfully", assets_count);
        }
        Ok(())
    }

    /// Find asset by app_id
    pub async fn find_by_app_id(&self, app_id: &str) -> Result<Option<Asset>, DbError> {
        let asset_model = Assets::find()
            .filter(assets::Column::AppId.eq(app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        match asset_model {
            Some(model) => Ok(Some(Asset {
                app_id: model.app_id,
                txid: model.txid,
                vout_index: model.vout_index,
                charm_id: model.charm_id,
                block_height: model.block_height as u64,
                date_created: model.date_created.naive_utc(),
                data: model.data,
                asset_type: model.asset_type,
                blockchain: model.blockchain,
                network: model.network,
                total_supply: model.total_supply,
            })),
            None => Ok(None),
        }
    }

    /// Get assets by charm_id
    pub async fn find_by_charm_id(&self, charm_id: &str) -> Result<Vec<Asset>, DbError> {
        let asset_models = Assets::find()
            .filter(assets::Column::CharmId.eq(charm_id))
            .all(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        let assets = asset_models
            .into_iter()
            .map(|model| Asset {
                app_id: model.app_id,
                txid: model.txid,
                vout_index: model.vout_index,
                charm_id: model.charm_id,
                block_height: model.block_height as u64,
                date_created: model.date_created.naive_utc(),
                data: model.data,
                asset_type: model.asset_type,
                blockchain: model.blockchain,
                network: model.network,
                total_supply: model.total_supply,
            })
            .collect();

        Ok(assets)
    }

    /// Update supply when charms are marked as spent
    /// Decrements supply for the given app_id and amount
    pub async fn update_supply_on_spent(
        &self,
        app_id: &str,
        amount: i64,
        asset_type: &str,
    ) -> Result<(), DbError> {
        // Extract hash for NFT-Token matching
        let hash = self.extract_hash_from_app_id(app_id);

        // Determine which asset to update
        let target_app_id = if asset_type == "token" {
            // For tokens, try to update parent NFT first
            let parent_nft_app_id = format!("n/{}", hash);
            let parent_nft = Assets::find()
                .filter(assets::Column::AppId.eq(&parent_nft_app_id))
                .one(&self.db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            if parent_nft.is_some() {
                parent_nft_app_id
            } else {
                // No parent NFT, update token asset directly
                app_id.to_string()
            }
        } else {
            app_id.to_string()
        };

        // Find and update the asset
        let asset = Assets::find()
            .filter(assets::Column::AppId.eq(&target_app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        if let Some(asset_model) = asset {
            let old_supply = asset_model.total_supply.unwrap_or(Decimal::ZERO);
            let amount_decimal = Decimal::from(amount);
            let new_supply = (old_supply - amount_decimal).max(Decimal::ZERO); // Prevent negative supply

            let update_model = assets::ActiveModel {
                id: Set(asset_model.id),
                total_supply: Set(Some(new_supply)),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };

            Assets::update(update_model)
                .exec(&self.db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;
        }

        Ok(())
    }

    /// Extract hash from app_id (removes t/ or n/ prefix)
    fn extract_hash_from_app_id(&self, app_id: &str) -> String {
        if let Some(stripped) = app_id.strip_prefix("t/") {
            stripped.split('/').next().unwrap_or(app_id).to_string()
        } else if let Some(stripped) = app_id.strip_prefix("n/") {
            stripped.split('/').next().unwrap_or(app_id).to_string()
        } else {
            app_id.to_string()
        }
    }
}
