use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
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

    /// Save a transaction
    pub async fn save_transaction(&self, transaction: &Transaction) -> Result<(), DbError> {
        // Check if transaction already exists
        if let Some(_existing) = self.get_by_txid(&transaction.txid).await? {
            // Transaction already exists, skip insertion
            return Ok(());
        }

        // Create a new active model
        let tx_model = transactions::ActiveModel {
            txid: Set(transaction.txid.clone()),
            block_height: Set(Some(transaction.block_height as i32)),
            ordinal: Set(transaction.ordinal),
            raw: Set(transaction.raw.clone()),
            charm: Set(transaction.charm.clone()),
            updated_at: Set(transaction.updated_at),
            status: Set(transaction.status.clone()),
            confirmations: Set(transaction.confirmations),
            blockchain: Set(transaction.blockchain.clone()),
            network: Set(transaction.network.clone()),
            mempool_detected_at: Set(None),
        };

        // Try to insert the transaction, handle duplicate key violations gracefully
        match tx_model.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Transaction already exists, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate the original error
                    Err(e.into())
                }
            }
        }
    }

    /// Get a transaction by its transaction ID
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<Transaction>, DbError> {
        // Query the database for the transaction
        let result = transactions::Entity::find_by_id(txid)
            .one(&self.conn)
            .await?;

        // Convert to domain model if found
        Ok(result.map(|t| self.to_domain_model(t)))
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

    /// Find transactions by blockchain and network
    pub async fn find_by_blockchain_network(
        &self,
        blockchain: &str,
        network: &str,
    ) -> Result<Vec<Transaction>, DbError> {
        // Query the database for transactions with the given blockchain and network
        let results = transactions::Entity::find()
            .filter(transactions::Column::Blockchain.eq(blockchain))
            .filter(transactions::Column::Network.eq(network))
            .order_by_desc(transactions::Column::BlockHeight)
            .all(&self.conn)
            .await?;

        // Convert to domain models
        Ok(results
            .into_iter()
            .map(|t| self.to_domain_model(t))
            .collect())
    }

    /// Find transactions with pagination
    pub async fn find_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Transaction>, u64), DbError> {
        // Create a paginator
        let paginator = transactions::Entity::find()
            .order_by_desc(transactions::Column::BlockHeight)
            .paginate(&self.conn, page_size);

        // Get the total number of pages
        let num_pages = paginator.num_pages().await?;

        // Get the current page
        let results = paginator.fetch_page(page).await?;

        // Convert to domain models
        let transactions = results
            .into_iter()
            .map(|t| self.to_domain_model(t))
            .collect();

        Ok((transactions, num_pages))
    }

    /// Save multiple transactions in a batch
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
        )>,
    ) -> Result<(), DbError> {
        // Skip individual existence checks - let database handle duplicates
        if transactions.is_empty() {
            return Ok(());
        }

        // Create active models for all transactions
        let now = chrono::Utc::now().naive_utc();
        let models: Vec<transactions::ActiveModel> = transactions
            .into_iter()
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
                )| {
                    let status = if is_confirmed {
                        "confirmed".to_string()
                    } else {
                        "pending".to_string()
                    };

                    transactions::ActiveModel {
                        txid: Set(txid),
                        block_height: Set(Some(block_height as i32)),
                        ordinal: Set(ordinal),
                        raw: Set(raw),
                        charm: Set(charm),
                        updated_at: Set(now),
                        status: Set(status),
                        confirmations: Set(confirmations),
                        blockchain: Set(blockchain),
                        network: Set(network),
                        mempool_detected_at: Set(None),
                    }
                },
            )
            .collect();

        // Try to insert all transactions, handle duplicate key violations gracefully
        match transactions::Entity::insert_many(models)
            .exec(&self.conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Some transactions already exist, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate the original error
                    Err(e.into())
                }
            }
        }
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

    // ==================== Fast Reindex Methods ====================

    /// Get the block range (min, max) for a given network
    pub async fn get_block_range(
        &self,
        network: &str,
    ) -> Result<(Option<u64>, Option<u64>), DbError> {
        use sea_orm::{FromQueryResult, Statement};

        #[derive(Debug, FromQueryResult)]
        struct BlockRange {
            min_block: Option<i32>,
            max_block: Option<i32>,
        }

        let result: Option<BlockRange> =
            BlockRange::find_by_statement(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"SELECT MIN(block_height) as min_block, MAX(block_height) as max_block 
               FROM transactions WHERE network = $1 AND block_height IS NOT NULL"#,
                [network.into()],
            ))
            .one(&self.conn)
            .await?;

        Ok(match result {
            Some(r) => (r.min_block.map(|v| v as u64), r.max_block.map(|v| v as u64)),
            None => (None, None),
        })
    }

    /// Get all distinct block heights with transactions for a network (sorted ascending)
    pub async fn get_blocks_with_transactions(&self, network: &str) -> Result<Vec<u64>, DbError> {
        use sea_orm::{FromQueryResult, Statement};

        #[derive(Debug, FromQueryResult)]
        struct BlockHeight {
            block_height: i32,
        }

        let results: Vec<BlockHeight> =
            BlockHeight::find_by_statement(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"SELECT DISTINCT block_height FROM transactions 
               WHERE network = $1 AND block_height IS NOT NULL 
               ORDER BY block_height ASC"#,
                [network.into()],
            ))
            .all(&self.conn)
            .await?;

        Ok(results.into_iter().map(|r| r.block_height as u64).collect())
    }

    /// Get transactions for a specific block (with hex data for reindexing)
    pub async fn get_transactions_for_reindex(
        &self,
        block_height: u64,
        network: &str,
    ) -> Result<Vec<(String, String, i64)>, DbError> {
        use sea_orm::{FromQueryResult, Statement};

        #[derive(Debug, FromQueryResult)]
        struct TxReindex {
            txid: String,
            hex: Option<String>,
            ordinal: i64,
        }

        let results: Vec<TxReindex> = TxReindex::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"SELECT txid, raw->>'hex' as hex, ordinal FROM transactions 
               WHERE block_height = $1 AND network = $2 
               ORDER BY ordinal ASC"#,
            [
                sea_orm::Value::Int(Some(block_height as i32)),
                network.into(),
            ],
        ))
        .all(&self.conn)
        .await?;

        Ok(results
            .into_iter()
            .filter_map(|r| r.hex.map(|h| (r.txid, h, r.ordinal)))
            .collect())
    }
}
