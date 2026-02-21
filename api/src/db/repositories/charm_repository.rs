// Charm database operations implementation

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use serde::Serialize;

use crate::db::error::DbError;
use crate::entity::charms;
use crate::models::PaginationParams;

/// Aggregated charm balance for a single app_id
#[derive(Debug, Serialize)]
pub struct CharmBalance {
    pub app_id: String,
    pub asset_type: String,
    pub confirmed_amount: i64,
    pub unconfirmed_amount: i64,
    pub confirmed_count: i64,
    pub unconfirmed_count: i64,
}

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
    /// NULLs FIRST so mempool charms (block_height=NULL) appear at the top
    pub async fn get_all_paginated_by_network(
        &self,
        pagination: &PaginationParams,
        network: &str,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        use sea_orm::{DbBackend, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct CountRow {
            count: i64,
        }

        let count_row = CountRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT COUNT(*) AS count FROM charms WHERE network = $1",
            [network.into()],
        ))
        .one(&self.conn)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        let total = count_row.map(|r| r.count as u64).unwrap_or(0);

        let offset = (pagination.page - 1) * pagination.limit;
        let charms = charms::Model::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT txid, vout, block_height, data, date_created, asset_type, blockchain,
                      network, address, spent, app_id, amount, mempool_detected_at, tags, verified
               FROM charms
               WHERE network = $1
               ORDER BY block_height DESC NULLS FIRST, date_created DESC
               LIMIT $2 OFFSET $3"#,
            [
                network.into(),
                (pagination.limit as i64).into(),
                (offset as i64).into(),
            ],
        ))
        .all(&self.conn)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

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

    /// Get charm balances by address, grouped by app_id
    /// Returns (app_id, asset_type, confirmed_amount, unconfirmed_amount, confirmed_count, unconfirmed_count)
    pub async fn get_charm_balances_by_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<Vec<CharmBalance>, DbError> {
        use sea_orm::{DbBackend, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct Row {
            app_id: String,
            asset_type: String,
            confirmed_amount: Option<rust_decimal::Decimal>,
            unconfirmed_amount: Option<rust_decimal::Decimal>,
            confirmed_count: Option<i64>,
            unconfirmed_count: Option<i64>,
        }

        let rows = Row::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            SELECT
                app_id,
                asset_type,
                SUM(CASE WHEN block_height IS NOT NULL AND block_height > 0 THEN amount ELSE 0 END) AS confirmed_amount,
                SUM(CASE WHEN block_height IS NULL OR block_height = 0 THEN amount ELSE 0 END) AS unconfirmed_amount,
                COUNT(CASE WHEN block_height IS NOT NULL AND block_height > 0 THEN 1 END) AS confirmed_count,
                COUNT(CASE WHEN block_height IS NULL OR block_height = 0 THEN 1 END) AS unconfirmed_count
            FROM charms
            WHERE address = $1 AND network = $2 AND spent = false
            GROUP BY app_id, asset_type
            ORDER BY app_id
            "#,
            [address.into(), network.into()],
        ))
        .all(&self.conn)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CharmBalance {
                app_id: r.app_id,
                asset_type: r.asset_type,
                confirmed_amount: r
                    .confirmed_amount
                    .map(|d| d.to_string().parse::<i64>().unwrap_or(0))
                    .unwrap_or(0),
                unconfirmed_amount: r
                    .unconfirmed_amount
                    .map(|d| d.to_string().parse::<i64>().unwrap_or(0))
                    .unwrap_or(0),
                confirmed_count: r.confirmed_count.unwrap_or(0),
                unconfirmed_count: r.unconfirmed_count.unwrap_or(0),
            })
            .collect())
    }

    /// Get all unspent charms for an address (individual rows, not aggregated)
    pub async fn get_unspent_charms_by_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Address.eq(address))
            .filter(charms::Column::Network.eq(network))
            .filter(charms::Column::Spent.eq(false))
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Get sibling charm app_ids for UTXOs owned by an address
    /// Returns (txid, vout) -> Vec<app_id> for all charms sharing those UTXOs
    /// Uses a single efficient SQL query with a subselect
    pub async fn get_sibling_app_ids_for_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<std::collections::HashMap<(String, i32), Vec<String>>, DbError> {
        use sea_orm::{DbBackend, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct Row {
            txid: String,
            vout: i32,
            app_id: String,
        }

        // Find all charms that share a (txid, vout) with unspent charms owned by this address
        let rows = Row::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            SELECT c2.txid, c2.vout, c2.app_id
            FROM charms c2
            WHERE (c2.txid, c2.vout) IN (
                SELECT txid, vout FROM charms
                WHERE address = $1 AND network = $2 AND spent = false
            )
            "#,
            [address.into(), network.into()],
        ))
        .all(&self.conn)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut map: std::collections::HashMap<(String, i32), Vec<String>> =
            std::collections::HashMap::new();
        for r in rows {
            map.entry((r.txid, r.vout)).or_default().push(r.app_id);
        }
        Ok(map)
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
