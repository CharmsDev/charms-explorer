//! Asset repository for database operations

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, NotSet};
use serde_json::Value;
use chrono::{DateTime, FixedOffset, Utc};

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

    /// Save a single asset to the database
    pub async fn save_asset(&self, asset: &Asset) -> Result<(), DbError> {
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
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
        };

        // Try to insert, if conflict occurs, update the existing record
        match Assets::insert(active_model).exec(&self.db).await {
            Ok(_) => {},
            Err(sea_orm::DbErr::RecordNotInserted) => {
                // Record already exists, update it
                let update_model = assets::ActiveModel {
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
                    created_at: NotSet,
                    updated_at: Set(Utc::now().into()),
                };
                
                Assets::update_many()
                    .filter(assets::Column::AppId.eq(&asset.app_id))
                    .set(update_model)
                    .exec(&self.db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;
            },
            Err(e) => return Err(DbError::SeaOrmError(e)),
        }

        Ok(())
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
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                }
            })
            .collect();

        // For batch operations, we'll insert and handle conflicts individually
        for active_model in active_models {
            match Assets::insert(active_model).exec(&self.db).await {
                Ok(_) => {},
                Err(sea_orm::DbErr::RecordNotInserted) => {
                    // Handle conflict by updating existing record
                    // For batch operations, we'll skip updates to keep it simple
                    // In production, you might want to implement proper batch upsert
                },
                Err(e) => return Err(DbError::SeaOrmError(e)),
            }
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
            })
            .collect();

        Ok(assets)
    }
}
