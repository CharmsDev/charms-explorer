// Charm database operations implementation

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::db::error::DbError;
use crate::entity::charms;

/// Repository for charm database operations
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Creates a new charm repository with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        CharmRepository { conn }
    }

    /// Retrieves a charm by transaction ID
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<charms::Model>, DbError> {
        charms::Entity::find_by_id(txid.to_string())
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Finds all charms with matching charm ID
    pub async fn find_by_charmid(&self, charmid: &str) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Charmid.eq(charmid))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Finds all charms with matching asset type
    pub async fn find_by_asset_type(
        &self,
        asset_type: &str,
    ) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Retrieves all charms ordered by descending block height
    pub async fn get_all(&self) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Retrieves all charm IDs, filtered by asset type if provided
    pub async fn get_charm_numbers_by_type(
        &self,
        asset_type: Option<&str>,
    ) -> Result<Vec<String>, DbError> {
        let mut query = charms::Entity::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(charms::Column::AssetType.eq(asset_type));
        }

        let charms = query
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await?;

        let charm_numbers = charms.into_iter().map(|c| c.charmid).collect();

        Ok(charm_numbers)
    }
}
