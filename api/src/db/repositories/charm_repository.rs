// Charm database operations implementation
// All queries use SeaORM ORM — no raw SQL.

use std::collections::HashMap;

use sea_orm::sea_query::{Expr, NullOrdering, Order};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
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
        let total = charms::Entity::find()
            .filter(charms::Column::Network.eq(network))
            .count(&self.conn)
            .await? as u64;

        let offset = (pagination.page - 1) * pagination.limit;
        let mut query = charms::Entity::find().filter(charms::Column::Network.eq(network));
        QuerySelect::query(&mut query)
            .order_by_with_nulls(
                charms::Column::BlockHeight,
                Order::Desc,
                NullOrdering::First,
            )
            .order_by(charms::Column::DateCreated, Order::Desc);
        let charms = query
            .limit(pagination.limit)
            .offset(offset)
            .all(&self.conn)
            .await?;

        Ok((charms, total))
    }

    /// Retrieves all charms paginated (all networks)
    /// NULLs FIRST so mempool charms (block_height=NULL) appear at the top
    pub async fn get_all_paginated(
        &self,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let total = charms::Entity::find().count(&self.conn).await? as u64;

        let offset = (pagination.page - 1) * pagination.limit;
        let mut query = charms::Entity::find();
        QuerySelect::query(&mut query)
            .order_by_with_nulls(
                charms::Column::BlockHeight,
                Order::Desc,
                NullOrdering::First,
            )
            .order_by(charms::Column::DateCreated, Order::Desc);
        let charms = query
            .limit(pagination.limit)
            .offset(offset)
            .all(&self.conn)
            .await?;

        Ok((charms, total))
    }

    /// Finds charms by asset type with pagination
    /// NULLs FIRST so mempool charms (block_height=NULL) appear at the top
    pub async fn find_by_asset_type_paginated(
        &self,
        asset_type: &str,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let total = charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .count(&self.conn)
            .await? as u64;

        let offset = (pagination.page - 1) * pagination.limit;
        let mut query = charms::Entity::find().filter(charms::Column::AssetType.eq(asset_type));
        QuerySelect::query(&mut query)
            .order_by_with_nulls(
                charms::Column::BlockHeight,
                Order::Desc,
                NullOrdering::First,
            )
            .order_by(charms::Column::DateCreated, Order::Desc);
        let charms = query
            .limit(pagination.limit)
            .offset(offset)
            .all(&self.conn)
            .await?;

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
    /// Uses SeaORM column_as + group_by for aggregation
    pub async fn get_charm_balances_by_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<Vec<CharmBalance>, DbError> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct Row {
            app_id: String,
            asset_type: String,
            confirmed_amount: Option<rust_decimal::Decimal>,
            unconfirmed_amount: Option<rust_decimal::Decimal>,
            confirmed_count: Option<i64>,
            unconfirmed_count: Option<i64>,
        }

        let rows = charms::Entity::find()
            .select_only()
            .column(charms::Column::AppId)
            .column(charms::Column::AssetType)
            .column_as(
                Expr::cust("SUM(CASE WHEN block_height IS NOT NULL AND block_height > 0 THEN amount ELSE 0 END)"),
                "confirmed_amount",
            )
            .column_as(
                Expr::cust("SUM(CASE WHEN block_height IS NULL OR block_height = 0 THEN amount ELSE 0 END)"),
                "unconfirmed_amount",
            )
            .column_as(
                Expr::cust("COUNT(CASE WHEN block_height IS NOT NULL AND block_height > 0 THEN 1 END)"),
                "confirmed_count",
            )
            .column_as(
                Expr::cust("COUNT(CASE WHEN block_height IS NULL OR block_height = 0 THEN 1 END)"),
                "unconfirmed_count",
            )
            .filter(charms::Column::Address.eq(address))
            .filter(charms::Column::Network.eq(network))
            .filter(charms::Column::Spent.eq(false))
            .group_by(charms::Column::AppId)
            .group_by(charms::Column::AssetType)
            .order_by_asc(charms::Column::AppId)
            .into_model::<Row>()
            .all(&self.conn)
            .await?;

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
    /// Two-step ORM approach: first get owned UTXOs, then find siblings
    pub async fn get_sibling_app_ids_for_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<HashMap<(String, i32), Vec<String>>, DbError> {
        // Step 1: Get all (txid, vout) pairs for unspent charms owned by this address
        let owned = charms::Entity::find()
            .filter(charms::Column::Address.eq(address))
            .filter(charms::Column::Network.eq(network))
            .filter(charms::Column::Spent.eq(false))
            .all(&self.conn)
            .await?;

        if owned.is_empty() {
            return Ok(HashMap::new());
        }

        // Collect unique (txid, vout) pairs
        let keys: Vec<(String, i32)> = owned.iter().map(|c| (c.txid.clone(), c.vout)).collect();
        let txids: Vec<String> = keys.iter().map(|(t, _)| t.clone()).collect();

        // Step 2: Get all charms sharing those txids (then filter by vout in memory)
        let siblings = charms::Entity::find()
            .filter(charms::Column::Txid.is_in(txids))
            .all(&self.conn)
            .await?;

        let key_set: std::collections::HashSet<(String, i32)> = keys.into_iter().collect();
        let mut map: HashMap<(String, i32), Vec<String>> = HashMap::new();
        for c in siblings {
            let key = (c.txid.clone(), c.vout);
            if key_set.contains(&key) {
                map.entry(key).or_default().push(c.app_id);
            }
        }
        Ok(map)
    }

    /// [RJJ-SUPPLY] Calculate circulating supply from unspent charms
    /// Returns SUM(amount) WHERE app_id LIKE 'prefix%' AND spent = false
    pub async fn get_circulating_supply_by_app_id_prefix(
        &self,
        app_id_prefix: &str,
    ) -> Result<Option<i64>, DbError> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct SupplyResult {
            total: Option<rust_decimal::Decimal>,
        }

        let pattern = format!("{}%", app_id_prefix);
        let result = charms::Entity::find()
            .select_only()
            .column_as(Expr::cust("SUM(amount)"), "total")
            .filter(charms::Column::AppId.starts_with(&pattern[..pattern.len() - 1]))
            .filter(charms::Column::Spent.eq(false))
            .into_model::<SupplyResult>()
            .one(&self.conn)
            .await?;

        Ok(result.and_then(|r| r.total.map(|d| d.to_string().parse::<i64>().unwrap_or(0))))
    }
}
