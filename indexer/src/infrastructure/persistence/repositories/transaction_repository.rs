use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::fmt;

use crate::domain::models::Transaction;
use crate::infrastructure::persistence::entities::transactions;
use crate::infrastructure::persistence::error::DbError;

/// Repository for transaction operations
#[derive(Clone)]
pub struct TransactionRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for TransactionRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionRepository")
            .finish_non_exhaustive()
    }
}

impl TransactionRepository {
    /// Create a new TransactionRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Find transactions by block height
    pub async fn find_by_block_height(&self, height: u64) -> Result<Vec<Transaction>, DbError> {
        // Query the database for transactions with the given block height
        let results = transactions::Entity::find()
            .filter(transactions::Column::BlockHeight.eq(height as i32))
            .order_by_asc(transactions::Column::Ordinal)
            .all(&self.conn)
            .await?;

        // Convert to domain models
        Ok(results
            .into_iter()
            .map(|t| self.to_domain_model(t))
            .collect())
    }

    /// Save multiple transactions in a batch.
    /// Uses ON CONFLICT DO UPDATE to promote pending/mempool transactions to
    /// confirmed status when the block processor re-encounters them.
    pub async fn save_batch(
        &self,
        transactions: Vec<(
            String,
            u64,
            i64,
            serde_json::Value,
            serde_json::Value,
            i32,
            bool,
            String,
            String,
            Option<String>,
            Option<String>,
        )>,
    ) -> Result<(), DbError> {
        if transactions.is_empty() {
            return Ok(());
        }

        use sea_orm::{ConnectionTrait, DbBackend, Statement};

        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let values: Vec<String> = transactions
            .iter()
            .map(
                |(
                    txid,
                    block_height,
                    ordinal,
                    raw,
                    charm,
                    confirmations,
                    is_confirmed,
                    blockchain,
                    network,
                    tags,
                    tx_type,
                )| {
                    let status = if *is_confirmed {
                        "confirmed"
                    } else {
                        "pending"
                    };
                    let raw_str = serde_json::to_string(raw).unwrap_or_else(|_| "{}".to_string());
                    let charm_str =
                        serde_json::to_string(charm).unwrap_or_else(|_| "{}".to_string());
                    let tags_sql = match tags {
                        Some(t) => format!("'{}'", t.replace('\'', "''")),
                        None => "NULL".to_string(),
                    };
                    let tx_type_sql = match tx_type {
                        Some(t) => format!("'{}'", t.replace('\'', "''")),
                        None => "NULL".to_string(),
                    };

                    format!(
                        "('{}', {}, {}, '{}'::jsonb, '{}'::jsonb, '{}', '{}', {}, '{}', '{}', {}, {})",
                        txid.replace('\'', "''"),
                        block_height,
                        ordinal,
                        raw_str.replace('\'', "''"),
                        charm_str.replace('\'', "''"),
                        now_str,
                        status,
                        confirmations,
                        blockchain.replace('\'', "''"),
                        network.replace('\'', "''"),
                        tags_sql,
                        tx_type_sql,
                    )
                },
            )
            .collect();

        let sql = format!(
            "INSERT INTO transactions (txid, block_height, ordinal, raw, charm, updated_at, status, confirmations, blockchain, network, tags, tx_type) \
             VALUES {} \
             ON CONFLICT (txid) DO UPDATE SET \
               block_height = COALESCE(EXCLUDED.block_height, transactions.block_height), \
               status = CASE WHEN EXCLUDED.block_height IS NOT NULL THEN 'confirmed' ELSE transactions.status END, \
               confirmations = GREATEST(EXCLUDED.confirmations, transactions.confirmations), \
               updated_at = EXCLUDED.updated_at, \
               charm = CASE WHEN EXCLUDED.charm != '{{}}'::jsonb THEN EXCLUDED.charm ELSE transactions.charm END, \
               raw = CASE WHEN EXCLUDED.raw != '{{}}'::jsonb THEN EXCLUDED.raw ELSE transactions.raw END, \
               tags = COALESCE(EXCLUDED.tags, transactions.tags), \
               tx_type = COALESCE(EXCLUDED.tx_type, transactions.tx_type)",
            values.join(", ")
        );

        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Convert a database entity to a domain model
    fn to_domain_model(&self, entity: transactions::Model) -> Transaction {
        Transaction::new(
            entity.txid,
            entity.block_height.unwrap_or(0) as u64,
            entity.ordinal,
            entity.raw,
            entity.charm,
            entity.updated_at,
            entity.confirmations,
            entity.status,
            entity.blockchain,
            entity.network,
        )
    }

}
