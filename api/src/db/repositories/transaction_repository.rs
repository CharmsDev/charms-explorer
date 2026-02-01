// Transaction database operations implementation
// [RJJ-SPELL] Repository to access the original spell data from transactions table

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::db::error::DbError;
use crate::entity::transactions;

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
    pub async fn get_spell_by_txid(&self, txid: &str) -> Result<Option<serde_json::Value>, DbError> {
        let tx = self.get_by_txid(txid).await?;
        Ok(tx.map(|t| t.charm))
    }
}
