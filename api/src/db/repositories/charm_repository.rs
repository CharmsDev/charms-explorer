// Charm database operations implementation

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::db::error::DbError;
use crate::entity::charms;
use crate::models::PaginationParams;

/// Repository for charm database operations
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Creates a new charm repository with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        CharmRepository { conn }
    }

    /// Returns a reference to the underlying database connection
    #[allow(dead_code)] // Used in charm_service for direct queries
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Retrieves charms by transaction ID (may return multiple due to composite key)
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Txid.eq(txid))
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Finds all charms with matching asset type
    #[allow(dead_code)]
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

    /// Retrieves all charms ordered by descending block height (legacy method)
    #[allow(dead_code)]
    pub async fn get_all(&self) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Retrieves all charms paginated by network
    pub async fn get_all_paginated_by_network(
        &self,
        pagination: &PaginationParams,
        network: &str,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let paginator = charms::Entity::find()
            .filter(charms::Column::Network.eq(network))
            .order_by_desc(charms::Column::BlockHeight)
            .paginate(&self.conn, pagination.limit);

        let total = paginator.num_items().await?;
        let charms = paginator.fetch_page(pagination.page - 1).await?;

        Ok((charms, total))
    }

    /// Retrieves all charms paginated
    pub async fn get_all_paginated(
        &self,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let paginator = charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .paginate(&self.conn, pagination.limit);

        let total = paginator.num_items().await?;
        let charms = paginator.fetch_page(pagination.page - 1).await?;

        Ok((charms, total))
    }

    /// Finds charms by asset type with pagination
    pub async fn find_by_asset_type_paginated(
        &self,
        asset_type: &str,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let paginator = charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .order_by_desc(charms::Column::BlockHeight)
            .paginate(&self.conn, pagination.limit);

        let total = paginator.num_items().await?;
        let charms = paginator.fetch_page(pagination.page - 1).await?;

        Ok((charms, total))
    }

    /// Finds charms by charm ID (app_id)
    pub async fn find_by_charmid(&self, charmid: &str) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::AppId.eq(charmid))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// [RJJ-ADDRESS-SEARCH] Finds UNSPENT charms by address
    /// Returns only charms where spent = false
    pub async fn find_by_address(&self, address: &str) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Address.eq(address))
            .filter(charms::Column::Spent.eq(false))
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

        let charm_numbers = charms.into_iter().map(|c| c.app_id).collect();

        Ok(charm_numbers)
    }

    /// Counts all charms in the database
    #[allow(dead_code)] // Reserved for future use
    pub async fn count_all(&self) -> Result<i64, DbError> {
        let count = charms::Entity::find().count(&self.conn).await?;

        Ok(count as i64)
    }

    /// Batch fetch charms by multiple txids (avoids N+1 queries)
    pub async fn get_by_txids(&self, txids: &[String]) -> Result<Vec<charms::Model>, DbError> {
        if txids.is_empty() {
            return Ok(vec![]);
        }
        charms::Entity::find()
            .filter(charms::Column::Txid.is_in(txids.to_vec()))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// [RJJ-SUPPLY] Calculate circulating supply from unspent charms
    /// This is the single source of truth for token supply
    /// Returns SUM(amount) WHERE app_id LIKE 'prefix%' AND spent = false
    pub async fn get_circulating_supply_by_app_id_prefix(
        &self,
        app_id_prefix: &str,
    ) -> Result<Option<i64>, DbError> {
        use sea_orm::{DbBackend, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct SupplyResult {
            total: Option<rust_decimal::Decimal>,
        }

        let result = SupplyResult::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT SUM(amount) as total FROM charms WHERE app_id LIKE $1 AND spent = false"#,
            [format!("{}%", app_id_prefix).into()],
        ))
        .one(&self.conn)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(result.and_then(|r| r.total.map(|d| d.to_string().parse::<i64>().unwrap_or(0))))
    }
}
