//! Repository for block_status operations
//! Unified block tracking for indexer control

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::fmt;

use crate::config::NetworkId;
use crate::infrastructure::persistence::entities::block_status;
use crate::infrastructure::persistence::error::DbError;

/// Repository for block_status operations
#[derive(Clone)]
pub struct BlockStatusRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for BlockStatusRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockStatusRepository")
            .finish_non_exhaustive()
    }
}

impl BlockStatusRepository {
    /// Create a new BlockStatusRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get pending blocks (downloaded but not processed)
    pub async fn get_pending_blocks(
        &self,
        network_id: &NetworkId,
        limit: u64,
    ) -> Result<Vec<i32>, DbError> {
        let results = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Downloaded.eq(true))
            .filter(block_status::Column::Processed.eq(false))
            .order_by_asc(block_status::Column::BlockHeight)
            .limit(limit)
            .all(&self.conn)
            .await?;

        Ok(results.into_iter().map(|b| b.block_height).collect())
    }

    /// Get the highest downloaded block height
    pub async fn get_last_downloaded_block(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<i32>, DbError> {
        let result = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Downloaded.eq(true))
            .order_by_desc(block_status::Column::BlockHeight)
            .one(&self.conn)
            .await?;

        Ok(result.map(|b| b.block_height))
    }

    /// Get the highest processed block height
    pub async fn get_last_processed_block(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<i32>, DbError> {
        let result = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Processed.eq(true))
            .order_by_desc(block_status::Column::BlockHeight)
            .one(&self.conn)
            .await?;

        Ok(result.map(|b| b.block_height))
    }

    /// Mark a block as downloaded
    pub async fn mark_downloaded(
        &self,
        block_height: i32,
        block_hash: Option<&str>,
        tx_count: i32,
        network_id: &NetworkId,
    ) -> Result<(), DbError> {
        let now = Utc::now();

        // Try to find existing record
        let existing = block_status::Entity::find()
            .filter(block_status::Column::BlockHeight.eq(block_height))
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            // Update existing record
            let mut update_model: block_status::ActiveModel = model.into();
            update_model.downloaded = Set(true);
            update_model.block_hash = Set(block_hash.map(|s| s.to_string()));
            update_model.tx_count = Set(Some(tx_count));
            update_model.downloaded_at = Set(Some(now.into()));
            update_model.updated_at = Set(now.into());
            update_model.update(&self.conn).await?;
        } else {
            // Insert new record
            let new_record = block_status::ActiveModel {
                block_height: Set(block_height),
                network: Set(network_id.name.clone()),
                blockchain: Set(network_id.blockchain_type()),
                downloaded: Set(true),
                processed: Set(false),
                confirmed: Set(false), // Will be updated when block is confirmed
                block_hash: Set(block_hash.map(|s| s.to_string())),
                tx_count: Set(Some(tx_count)),
                charm_count: Set(None),
                downloaded_at: Set(Some(now.into())),
                processed_at: Set(None),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };
            new_record.insert(&self.conn).await?;
        }

        Ok(())
    }

    /// Mark a block as processed
    pub async fn mark_processed(
        &self,
        block_height: i32,
        charm_count: i32,
        network_id: &NetworkId,
    ) -> Result<(), DbError> {
        let now = Utc::now();

        let existing = block_status::Entity::find()
            .filter(block_status::Column::BlockHeight.eq(block_height))
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            let mut update_model: block_status::ActiveModel = model.into();
            update_model.processed = Set(true);
            update_model.charm_count = Set(Some(charm_count));
            update_model.processed_at = Set(Some(now.into()));
            update_model.updated_at = Set(now.into());
            update_model.update(&self.conn).await?;
        } else {
            return Err(DbError::Other(format!(
                "Block {} not found for network {}",
                block_height, network_id.name
            )));
        }

        Ok(())
    }

    /// Count pending blocks
    pub async fn count_pending_blocks(&self, network_id: &NetworkId) -> Result<i64, DbError> {
        let count = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Downloaded.eq(true))
            .filter(block_status::Column::Processed.eq(false))
            .count(&self.conn)
            .await?;

        Ok(count as i64)
    }

    /// Reset all blocks to unprocessed (for reindex)
    /// Only resets CONFIRMED blocks - unconfirmed blocks should not be reset
    pub async fn reset_all_processed(&self, network_id: &NetworkId) -> Result<u64, DbError> {
        use sea_orm::sea_query::Expr;

        let result = block_status::Entity::update_many()
            .col_expr(block_status::Column::Processed, Expr::value(false))
            .col_expr(
                block_status::Column::ProcessedAt,
                Expr::value::<Option<chrono::DateTime<Utc>>>(None),
            )
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Confirmed.eq(true)) // Only reset confirmed blocks
            .exec(&self.conn)
            .await?;

        Ok(result.rows_affected)
    }

    /// Mark a block as confirmed
    pub async fn mark_confirmed(
        &self,
        block_height: i32,
        network_id: &NetworkId,
    ) -> Result<(), DbError> {
        let now = Utc::now();

        let existing = block_status::Entity::find()
            .filter(block_status::Column::BlockHeight.eq(block_height))
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            let mut update_model: block_status::ActiveModel = model.into();
            update_model.confirmed = Set(true);
            update_model.updated_at = Set(now.into());
            update_model.update(&self.conn).await?;
        }

        Ok(())
    }

    /// Get pending blocks for REINDEX mode (only confirmed, downloaded, not processed)
    pub async fn get_pending_confirmed_blocks(
        &self,
        network_id: &NetworkId,
        limit: u64,
    ) -> Result<Vec<i32>, DbError> {
        let results = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Downloaded.eq(true))
            .filter(block_status::Column::Processed.eq(false))
            .filter(block_status::Column::Confirmed.eq(true)) // Only confirmed blocks
            .order_by_asc(block_status::Column::BlockHeight)
            .limit(limit)
            .all(&self.conn)
            .await?;

        Ok(results.into_iter().map(|b| b.block_height).collect())
    }

    /// Get unconfirmed blocks (mempool/live processing)
    pub async fn get_unconfirmed_blocks(
        &self,
        network_id: &NetworkId,
    ) -> Result<Vec<i32>, DbError> {
        let results = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Confirmed.eq(false))
            .order_by_asc(block_status::Column::BlockHeight)
            .all(&self.conn)
            .await?;

        Ok(results.into_iter().map(|b| b.block_height).collect())
    }

    /// Get the highest confirmed block height
    pub async fn get_last_confirmed_block(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<i32>, DbError> {
        let result = block_status::Entity::find()
            .filter(block_status::Column::Network.eq(network_id.name.clone()))
            .filter(block_status::Column::Blockchain.eq(network_id.blockchain_type()))
            .filter(block_status::Column::Confirmed.eq(true))
            .order_by_desc(block_status::Column::BlockHeight)
            .one(&self.conn)
            .await?;

        Ok(result.map(|b| b.block_height))
    }
}
