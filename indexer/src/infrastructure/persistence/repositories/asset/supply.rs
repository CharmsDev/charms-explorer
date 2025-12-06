//! Supply management operations for assets

use super::helpers;
use crate::infrastructure::persistence::entities::{assets, prelude::*};
use crate::infrastructure::persistence::error::DbError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

/// [RJJ-SUPPLY] Update supply when charms are marked as spent
/// Decrements supply for the given app_id and amount
pub async fn update_supply_on_spent(
    db: &DatabaseConnection,
    app_id: &str,
    amount: i64,
    asset_type: &str,
) -> Result<(), DbError> {
    // Extract hash for NFT-Token matching
    let hash = helpers::extract_hash_from_app_id(app_id);

    // Determine which asset to update
    let target_app_id = if asset_type == "token" {
        // For tokens, try to update parent NFT first
        let parent_nft_app_id = format!("n/{}", hash);
        let parent_nft = Assets::find()
            .filter(assets::Column::AppId.eq(&parent_nft_app_id))
            .one(db)
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
        .one(db)
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
            .exec(db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;
    }

    Ok(())
}
