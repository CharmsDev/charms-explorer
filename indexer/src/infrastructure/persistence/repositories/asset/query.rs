//! Query operations for asset repository

use super::helpers;
use crate::domain::models::Asset;
use crate::infrastructure::persistence::entities::{assets, prelude::*};
use crate::infrastructure::persistence::error::DbError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

/// Find asset by app_id
pub async fn find_by_app_id(
    db: &DatabaseConnection,
    app_id: &str,
) -> Result<Option<Asset>, DbError> {
    let asset_model = Assets::find()
        .filter(assets::Column::AppId.eq(app_id))
        .one(db)
        .await
        .map_err(|e| DbError::SeaOrmError(e))?;

    Ok(asset_model.map(helpers::to_domain_model))
}

/// Get assets by charm_id
pub async fn find_by_charm_id(
    db: &DatabaseConnection,
    charm_id: &str,
) -> Result<Vec<Asset>, DbError> {
    let asset_models = Assets::find()
        .filter(assets::Column::CharmId.eq(charm_id))
        .all(db)
        .await
        .map_err(|e| DbError::SeaOrmError(e))?;

    Ok(asset_models
        .into_iter()
        .map(helpers::to_domain_model)
        .collect())
}
