// [RJJ-DEX] Repository for DEX orders queries

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::db::DbError;
use crate::entity::dex_orders;

#[derive(Clone, Debug)]
pub struct DexOrdersRepository {
    conn: DatabaseConnection,
}

impl DexOrdersRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get all open orders (status = 'open'), optionally filtered by asset, side, network
    pub async fn find_open_orders(
        &self,
        asset_app_id: Option<&str>,
        side: Option<&str>,
        network: Option<&str>,
    ) -> Result<Vec<dex_orders::Model>, DbError> {
        let mut query = dex_orders::Entity::find()
            .filter(dex_orders::Column::Status.eq("open"));

        if let Some(asset) = asset_app_id {
            query = query.filter(dex_orders::Column::AssetAppId.eq(asset));
        }
        if let Some(s) = side {
            query = query.filter(dex_orders::Column::Side.eq(s));
        }
        if let Some(n) = network {
            query = query.filter(dex_orders::Column::Network.eq(n));
        }

        let results = query
            .order_by_desc(dex_orders::Column::CreatedAt)
            .all(&self.conn)
            .await?;
        Ok(results)
    }

    /// Get order by ID
    pub async fn get_by_id(&self, order_id: &str) -> Result<Option<dex_orders::Model>, DbError> {
        let result = dex_orders::Entity::find_by_id(order_id.to_string())
            .one(&self.conn)
            .await?;
        Ok(result)
    }

    /// Find all orders by asset (any status)
    pub async fn find_by_asset(&self, asset_app_id: &str) -> Result<Vec<dex_orders::Model>, DbError> {
        let results = dex_orders::Entity::find()
            .filter(dex_orders::Column::AssetAppId.eq(asset_app_id))
            .order_by_desc(dex_orders::Column::CreatedAt)
            .all(&self.conn)
            .await?;
        Ok(results)
    }

    /// Find orders by maker address
    pub async fn find_by_maker(
        &self,
        maker: &str,
        status: Option<&str>,
    ) -> Result<Vec<dex_orders::Model>, DbError> {
        let mut query = dex_orders::Entity::find()
            .filter(dex_orders::Column::Maker.eq(maker));

        if let Some(s) = status {
            query = query.filter(dex_orders::Column::Status.eq(s));
        }

        let results = query
            .order_by_desc(dex_orders::Column::CreatedAt)
            .all(&self.conn)
            .await?;
        Ok(results)
    }
}
