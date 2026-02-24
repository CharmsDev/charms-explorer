// Transaction database operations implementation
// [RJJ-SPELL] Repository to access the original spell data from transactions table
// All queries use SeaORM ORM — no raw SQL.

use crate::db::error::DbError;
use crate::entity::transactions;
use crate::models::PaginationParams;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

/// Repository for transaction database operations
pub struct TransactionRepository {
    conn: DatabaseConnection,
}

impl TransactionRepository {
    /// Creates a new transaction repository with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        TransactionRepository { conn }
    }

    /// Retrieves a transaction by txid, including the original spell data
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<transactions::Model>, DbError> {
        transactions::Entity::find()
            .filter(transactions::Column::Txid.eq(txid))
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Get just the spell (charm field) for a transaction
    pub async fn get_spell_by_txid(
        &self,
        txid: &str,
    ) -> Result<Option<serde_json::Value>, DbError> {
        let tx = self.get_by_txid(txid).await?;
        Ok(tx.map(|t| t.charm))
    }

    /// Retrieves all transactions paginated, ordered by block_height DESC
    pub async fn get_all_paginated(
        &self,
        pagination: &PaginationParams,
    ) -> Result<(Vec<transactions::Model>, u64), DbError> {
        let total = transactions::Entity::find().count(&self.conn).await? as u64;

        let offset = (pagination.page - 1) * pagination.limit;
        let txs = transactions::Entity::find()
            .order_by_desc(transactions::Column::BlockHeight)
            .order_by_desc(transactions::Column::UpdatedAt)
            .limit(pagination.limit)
            .offset(offset)
            .all(&self.conn)
            .await?;

        Ok((txs, total))
    }

    /// Retrieves transactions paginated, filtered by network
    pub async fn get_all_paginated_by_network(
        &self,
        pagination: &PaginationParams,
        network: &str,
    ) -> Result<(Vec<transactions::Model>, u64), DbError> {
        let total = transactions::Entity::find()
            .filter(transactions::Column::Network.eq(network))
            .count(&self.conn)
            .await? as u64;

        let offset = (pagination.page - 1) * pagination.limit;
        let txs = transactions::Entity::find()
            .filter(transactions::Column::Network.eq(network))
            .order_by_desc(transactions::Column::BlockHeight)
            .order_by_desc(transactions::Column::UpdatedAt)
            .limit(pagination.limit)
            .offset(offset)
            .all(&self.conn)
            .await?;

        Ok((txs, total))
    }
}
