use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
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
    pub async fn get_summary(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<summary::Model>, DbError> {
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
        )
        .await
    }

    /// Update summary statistics for a network with bitcoin node information
    #[allow(clippy::too_many_arguments)]
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
                bitcoin_node_status: Set(
                    bitcoin_node_status.unwrap_or_else(|| "unknown".to_string())
                ),
                bitcoin_node_block_count: Set(bitcoin_node_block_count.unwrap_or(0)),
                bitcoin_node_best_block_hash: Set(
                    bitcoin_node_best_block_hash.unwrap_or_else(|| "unknown".to_string())
                ),
                last_updated: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                // Tag statistics [RJJ-DEX]
                charms_cast_count: Set(0),
                bro_count: Set(0),
                dex_orders_count: Set(0),
            };

            new_summary.insert(&self.conn).await?;
        }

        Ok(())
    }

    /// Get database connection for direct queries
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Get tag statistics from charms table (counts by tag)
    /// Returns (charms_cast_count, bro_count, dex_orders_count)
    pub async fn get_tag_stats(&self, network: &str) -> Result<(i64, i64, i64), DbError> {
        use sea_orm::FromQueryResult;

        #[derive(Debug, FromQueryResult)]
        struct TagCount {
            count: i64,
        }

        // Count charms with 'charms-cast' tag
        let cast_result: Option<TagCount> = TagCount::find_by_statement(
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM charms WHERE tags LIKE '%charms-cast%' AND network = $1",
                vec![network.into()],
            )
        )
        .one(&self.conn)
        .await?;
        let charms_cast_count = cast_result.map(|r| r.count).unwrap_or(0);

        // Count charms with 'bro' tag
        let bro_result: Option<TagCount> =
            TagCount::find_by_statement(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM charms WHERE tags LIKE '%bro%' AND network = $1",
                vec![network.into()],
            ))
            .one(&self.conn)
            .await?;
        let bro_count = bro_result.map(|r| r.count).unwrap_or(0);

        // Count dex_orders
        let dex_result: Option<TagCount> =
            TagCount::find_by_statement(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM dex_orders WHERE network = $1",
                vec![network.into()],
            ))
            .one(&self.conn)
            .await?;
        let dex_orders_count = dex_result.map(|r| r.count).unwrap_or(0);

        Ok((charms_cast_count, bro_count, dex_orders_count))
    }

    /// Update tag statistics in summary table
    pub async fn update_tag_stats(
        &self,
        network_id: &NetworkId,
        charms_cast_count: i64,
        bro_count: i64,
        dex_orders_count: i64,
    ) -> Result<(), DbError> {
        let existing = summary::Entity::find()
            .filter(summary::Column::Network.eq(network_id.name.clone()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            let mut update_model: summary::ActiveModel = model.into();
            update_model.charms_cast_count = Set(charms_cast_count);
            update_model.bro_count = Set(bro_count);
            update_model.dex_orders_count = Set(dex_orders_count);
            update_model.updated_at = Set(Utc::now());
            update_model.update(&self.conn).await?;
        }

        Ok(())
    }
}
