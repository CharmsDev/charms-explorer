//! Asset repository for database operations

use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, Set, NotSet};
use chrono::{DateTime, FixedOffset, Utc};
use serde_json::Value;
use rust_decimal::Decimal;

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
                    date_created: Set(DateTime::<FixedOffset>::from(DateTime::<Utc>::from_naive_utc_and_offset(asset.date_created, Utc))),
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

        println!("💾 Batch saving {} assets to database", assets.len());

        let now = Utc::now();
        let active_models: Vec<assets::ActiveModel> = assets
            .into_iter()
            .map(|(app_id, txid, vout_index, charm_id, block_height, data, asset_type, blockchain, network)| {
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
                    name: NotSet,
                    symbol: NotSet,
                    description: NotSet,
                    image_url: NotSet,
                    total_supply: Set(Some(Decimal::from(1))),
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                }
            })
            .collect();

        // For batch operations, we'll insert and handle conflicts individually
        let mut saved_count = 0;
        for active_model in active_models {
            match Assets::insert(active_model).exec(&self.db).await {
                Ok(_) => {
                    saved_count += 1;
                },
                Err(sea_orm::DbErr::RecordNotInserted) => {
                    // Handle conflict by updating existing record
                    // For batch operations, we'll skip updates to keep it simple
                    // In production, you might want to implement proper batch upsert
                },
                Err(e) => return Err(DbError::SeaOrmError(e)),
            }
        }
        
        println!("✅ Batch saved {} assets successfully", saved_count);

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
}
