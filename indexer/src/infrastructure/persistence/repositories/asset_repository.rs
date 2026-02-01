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
        // Check if asset already exists
        let existing_asset = Assets::find()
            .filter(assets::Column::AppId.eq(&asset.app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        match existing_asset {
            Some(existing) => {
                // Asset exists, update supply
                let old_supply = existing.total_supply.unwrap_or(Decimal::ZERO);
                let amount_decimal = Decimal::from(amount);
                let new_supply = old_supply + amount_decimal;

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
            }
            None => {
                // Asset doesn't exist, create new one
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
                    decimals: Set(8),
                    is_reference_nft: Set(false),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                };

                Assets::insert(active_model)
                    .exec(&self.db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;
            }
        }

        Ok(())
    }

    /// Save a single asset to the database (legacy method)
    pub async fn save_asset(&self, asset: &Asset) -> Result<(), DbError> {
        self.save_or_update_asset(asset, 1).await
    }

    /// Save multiple assets in a batch operation
    /// Delegates to asset/save.rs which handles NFT-token metadata inheritance
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
        crate::infrastructure::persistence::repositories::asset::save::save_batch(&self.db, assets)
            .await
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
    pub async fn update_supply_on_spent(
        &self,
        app_id: &str,
        amount: i64,
        asset_type: &str,
    ) -> Result<(), DbError> {
        let hash = self.extract_hash_from_app_id(app_id);

        let target_app_id = if asset_type == "token" {
            let parent_nft_pattern = format!("n/{}/%", hash);
            let parent_nft = Assets::find()
                .filter(assets::Column::AssetType.eq("nft"))
                .filter(assets::Column::AppId.like(&parent_nft_pattern))
                .one(&self.db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            if let Some(nft) = parent_nft {
                nft.app_id
            } else {
                app_id.to_string()
            }
        } else {
            app_id.to_string()
        };

        let asset = Assets::find()
            .filter(assets::Column::AppId.eq(&target_app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        if let Some(asset_model) = asset {
            let old_supply = asset_model.total_supply.unwrap_or(Decimal::ZERO);
            let amount_decimal = Decimal::from(amount);
            let new_supply = (old_supply - amount_decimal).max(Decimal::ZERO);

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

    /// Extract hash from app_id (removes t/ or n/ prefix and returns only the 64-char hash)
    fn extract_hash_from_app_id(&self, app_id: &str) -> String {
        let without_prefix = if let Some(stripped) = app_id.strip_prefix("t/") {
            stripped
        } else if let Some(stripped) = app_id.strip_prefix("n/") {
            stripped
        } else {
            app_id
        };

        // Hash is 64 characters
        if without_prefix.len() >= 64 {
            without_prefix[..64].to_string()
        } else {
            without_prefix.to_string()
        }
    }

    /// Mark NFT as reference and update token with inherited name
    pub async fn mark_nft_as_reference_and_get_name(
        &self,
        nft_pattern: &str,
        token_app_id: &str,
    ) -> Result<(), DbError> {
        // Find parent NFT by pattern
        let parent_nft = Assets::find()
            .filter(assets::Column::AssetType.eq("nft"))
            .filter(assets::Column::AppId.like(nft_pattern))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        if let Some(nft) = parent_nft {
            // Mark NFT as reference
            let mut nft_active: assets::ActiveModel = nft.clone().into();
            nft_active.is_reference_nft = Set(true);
            nft_active.updated_at = Set(Utc::now().into());
            Assets::update(nft_active)
                .exec(&self.db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            // Update token with inherited name
            if let Some(name) = nft.name {
                let token = Assets::find()
                    .filter(assets::Column::AppId.eq(token_app_id))
                    .one(&self.db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;

                if let Some(token_model) = token {
                    let mut token_active: assets::ActiveModel = token_model.into();
                    token_active.name = Set(Some(name));
                    token_active.updated_at = Set(Utc::now().into());
                    Assets::update(token_active)
                        .exec(&self.db)
                        .await
                        .map_err(|e| DbError::SeaOrmError(e))?;
                }
            }
        }

        Ok(())
    }

    /// Update NFT metadata (name, image_url) directly
    pub async fn update_nft_metadata(
        &self,
        app_id: &str,
        name: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<(), DbError> {
        let existing = Assets::find()
            .filter(assets::Column::AppId.eq(app_id))
            .one(&self.db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        if let Some(asset) = existing {
            let mut active: assets::ActiveModel = asset.into();

            if let Some(n) = name {
                active.name = Set(Some(n.to_string()));
            }
            if let Some(img) = image_url {
                active.image_url = Set(Some(img.to_string()));
            }
            active.updated_at = Set(Utc::now().into());

            active
                .update(&self.db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;
        }

        Ok(())
    }
}
