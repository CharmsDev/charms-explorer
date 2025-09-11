use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use std::fmt;

use crate::config::NetworkId;
use crate::infrastructure::persistence::entities::summary;
use crate::infrastructure::persistence::error::DbError;

/// Repository for summary operations
#[derive(Clone)]
pub struct SummaryRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for SummaryRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SummaryRepository").finish_non_exhaustive()
    }
}

impl SummaryRepository {
    /// Create a new SummaryRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get summary for a specific network
    pub async fn get_summary(&self, network_id: &NetworkId) -> Result<Option<summary::Model>, DbError> {
        let result = summary::Entity::find()
            .filter(summary::Column::Network.eq(network_id.name.clone()))
            .one(&self.conn)
            .await?;

        Ok(result)
    }

    /// Update summary statistics for a network
    pub async fn update_summary(
        &self,
        network_id: &NetworkId,
        last_processed_block: i32,
        latest_confirmed_block: i32,
        total_charms: i64,
        total_transactions: i64,
        confirmed_transactions: i64,
        nft_count: i64,
        token_count: i64,
        dapp_count: i64,
        other_count: i64,
    ) -> Result<(), DbError> {
        self.update_summary_with_bitcoin_node(
            network_id,
            last_processed_block,
            latest_confirmed_block,
            total_charms,
            total_transactions,
            confirmed_transactions,
            nft_count,
            token_count,
            dapp_count,
            other_count,
            None,
            None,
            None,
        ).await
    }

    /// Update summary statistics for a network with bitcoin node information
    pub async fn update_summary_with_bitcoin_node(
        &self,
        network_id: &NetworkId,
        last_processed_block: i32,
        latest_confirmed_block: i32,
        total_charms: i64,
        total_transactions: i64,
        confirmed_transactions: i64,
        nft_count: i64,
        token_count: i64,
        dapp_count: i64,
        other_count: i64,
        bitcoin_node_status: Option<String>,
        bitcoin_node_block_count: Option<i64>,
        bitcoin_node_best_block_hash: Option<String>,
    ) -> Result<(), DbError> {
        let confirmation_rate = if total_transactions > 0 {
            ((confirmed_transactions as f64 / total_transactions as f64) * 100.0) as i32
        } else {
            0
        };

        let now = Utc::now();

        // Try to find existing summary
        let existing = summary::Entity::find()
            .filter(summary::Column::Network.eq(network_id.name.clone()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            // Update existing summary
            let mut update_model: summary::ActiveModel = model.into();
            update_model.last_processed_block = Set(last_processed_block);
            update_model.latest_confirmed_block = Set(latest_confirmed_block);
            update_model.total_charms = Set(total_charms);
            update_model.total_transactions = Set(total_transactions);
            update_model.confirmed_transactions = Set(confirmed_transactions);
            update_model.confirmation_rate = Set(confirmation_rate);
            update_model.nft_count = Set(nft_count);
            update_model.token_count = Set(token_count);
            update_model.dapp_count = Set(dapp_count);
            update_model.other_count = Set(other_count);
            
            // Update bitcoin node info if provided
            if let Some(status) = bitcoin_node_status {
                update_model.bitcoin_node_status = Set(status);
            }
            if let Some(block_count) = bitcoin_node_block_count {
                update_model.bitcoin_node_block_count = Set(block_count);
            }
            if let Some(best_block_hash) = bitcoin_node_best_block_hash {
                update_model.bitcoin_node_best_block_hash = Set(best_block_hash);
            }
            
            update_model.last_updated = Set(now);
            update_model.updated_at = Set(now);

            update_model.update(&self.conn).await?;
        } else {
            // Create new summary
            let new_summary = summary::ActiveModel {
                id: sea_orm::NotSet,
                network: Set(network_id.name.clone()),
                last_processed_block: Set(last_processed_block),
                latest_confirmed_block: Set(latest_confirmed_block),
                total_charms: Set(total_charms),
                total_transactions: Set(total_transactions),
                confirmed_transactions: Set(confirmed_transactions),
                confirmation_rate: Set(confirmation_rate),
                nft_count: Set(nft_count),
                token_count: Set(token_count),
                dapp_count: Set(dapp_count),
                other_count: Set(other_count),
                bitcoin_node_status: Set(bitcoin_node_status.unwrap_or_else(|| "unknown".to_string())),
                bitcoin_node_block_count: Set(bitcoin_node_block_count.unwrap_or(0)),
                bitcoin_node_best_block_hash: Set(bitcoin_node_best_block_hash.unwrap_or_else(|| "unknown".to_string())),
                last_updated: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
            };

            new_summary.insert(&self.conn).await?;
        }

        Ok(())
    }

    /// Get database connection for direct queries
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.conn
    }
}
