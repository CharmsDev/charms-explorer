use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::domain::models::Transaction;
use crate::infrastructure::persistence::entities::transactions;
use crate::infrastructure::persistence::error::DbError;

/// Repository for transaction operations
#[derive(Clone)]
pub struct TransactionRepository {
    conn: DatabaseConnection,
}

impl TransactionRepository {
    /// Create a new TransactionRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Save a transaction
    pub async fn save_transaction(&self, transaction: &Transaction) -> Result<(), DbError> {
        // Create a new active model
        let tx_model = transactions::ActiveModel {
            txid: Set(transaction.txid.clone()),
            block_height: Set(transaction.block_height as i32),
            ordinal: Set(transaction.ordinal),
            raw: Set(transaction.raw.clone()),
            charm: Set(transaction.charm.clone()),
            updated_at: Set(transaction.updated_at),
            status: Set(transaction.status.clone()),
            confirmations: Set(transaction.confirmations),
        };

        // Insert or update the transaction
        tx_model.insert(&self.conn).await?;

        Ok(())
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
        )>,
    ) -> Result<(), DbError> {
        // Create active models for each transaction
        let now = chrono::Utc::now().naive_utc();
        let models: Vec<transactions::ActiveModel> = transactions
            .into_iter()
            .map(
                |(txid, block_height, ordinal, raw, charm, confirmations, is_confirmed)| {
                    let status = if is_confirmed {
                        "confirmed".to_string()
                    } else {
                        "pending".to_string()
                    };

                    transactions::ActiveModel {
                        txid: Set(txid),
                        block_height: Set(block_height as i32),
                        ordinal: Set(ordinal),
                        raw: Set(raw),
                        charm: Set(charm),
                        updated_at: Set(now),
                        status: Set(status),
                        confirmations: Set(confirmations),
                    }
                },
            )
            .collect();

        // Insert all transactions
        transactions::Entity::insert_many(models)
            .exec(&self.conn)
            .await?;

        Ok(())
    }

    /// Convert a database entity to a domain model
    fn to_domain_model(&self, entity: transactions::Model) -> Transaction {
        Transaction::new(
            entity.txid,
            entity.block_height as u64,
            entity.ordinal,
            entity.raw,
            entity.charm,
            entity.updated_at,
            entity.confirmations,
            entity.status,
        )
    }
}
