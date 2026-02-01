use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::fmt;

use crate::config::NetworkId;
use crate::infrastructure::persistence::entities::bookmark;
use crate::infrastructure::persistence::error::DbError;

/// Repository for bookmark operations
#[derive(Clone)]
pub struct BookmarkRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for BookmarkRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BookmarkRepository").finish_non_exhaustive()
    }
}

impl BookmarkRepository {
    /// Create a new BookmarkRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get the last processed block for a specific network
    pub async fn get_last_processed_block(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<u64>, DbError> {
        // Query the database for the last processed block for the specified network
        let result = bookmark::Entity::find()
            .filter(bookmark::Column::Network.eq(network_id.name.clone()))
            .filter(bookmark::Column::Blockchain.eq(network_id.blockchain_type()))
            .order_by_desc(bookmark::Column::Height)
            .one(&self.conn)
            .await?;

        // Return the height if found
        Ok(result.map(|b| b.height as u64))
    }

    /// Save a bookmark for a specific network
    pub async fn save_bookmark(
        &self,
        hash: &str,
        height: u64,
        is_confirmed: bool,
        network_id: &NetworkId,
    ) -> Result<(), DbError> {
        // Create a new active model
        let status = if is_confirmed {
            "confirmed".to_string()
        } else {
            "pending".to_string()
        };

        // Get blockchain type as a string
        let blockchain_type = network_id.blockchain_type();

        let bookmark = bookmark::ActiveModel {
            hash: Set(hash.to_string()),
            height: Set(height as i32),
            status: Set(status.clone()),
            last_updated_at: Set(Utc::now().into()),
            network: Set(network_id.name.clone()),
            blockchain: Set(blockchain_type),
        };

        // Try to insert the bookmark, if it fails with a unique constraint violation,
        // then update the existing bookmark instead
        match bookmark.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Update the existing bookmark
                    let existing = bookmark::Entity::find()
                        .filter(bookmark::Column::Hash.eq(hash.to_string()))
                        .filter(bookmark::Column::Network.eq(network_id.name.clone()))
                        .filter(bookmark::Column::Blockchain.eq(network_id.blockchain_type()))
                        .one(&self.conn)
                        .await?;

                    if let Some(model) = existing {
                        let mut update_model: bookmark::ActiveModel = model.into();
                        update_model.status = Set(status.clone());
                        update_model.last_updated_at = Set(Utc::now().into());
                        update_model.update(&self.conn).await?;
                        Ok(())
                    } else {
                        // This shouldn't happen, but just in case
                        Err(DbError::Other(format!(
                            "Failed to find bookmark for update: hash={}, network={}, blockchain={}",
                            hash,
                            network_id.name,
                            network_id.blockchain_type()
                        )))
                    }
                } else {
                    // If it's not a duplicate key error, propagate the original error
                    Err(e.into())
                }
            }
        }
    }

    /// Get the last updated timestamp for a specific network
    pub async fn get_last_updated_timestamp(
        &self,
        network_id: &NetworkId,
    ) -> Result<Option<DateTime<Utc>>, DbError> {
        // Query the database for the last updated bookmark for the specified network
        let result = bookmark::Entity::find()
            .filter(bookmark::Column::Network.eq(network_id.name.clone()))
            .filter(bookmark::Column::Blockchain.eq(network_id.blockchain_type()))
            .order_by_desc(bookmark::Column::LastUpdatedAt)
            .one(&self.conn)
            .await?;

        // Return the timestamp if found
        Ok(result.map(|b| b.last_updated_at.into()))
    }

    /// Get the last updated timestamp for any network
    pub async fn get_last_updated_timestamp_any_network(
        &self,
    ) -> Result<Option<DateTime<Utc>>, DbError> {
        // Query the database for the last updated bookmark across all networks
        let result = bookmark::Entity::find()
            .order_by_desc(bookmark::Column::LastUpdatedAt)
            .one(&self.conn)
            .await?;

        // Return the timestamp if found
        Ok(result.map(|b| b.last_updated_at.into()))
    }

    /// Update bookmark height for a network (used by fast reindexer)
    /// This updates the existing bookmark rather than creating a new one
    pub async fn update_bookmark_height(
        &self,
        new_height: u64,
        network_id: &NetworkId,
    ) -> Result<(), DbError> {
        // Find existing bookmark for this network
        let existing = bookmark::Entity::find()
            .filter(bookmark::Column::Network.eq(network_id.name.clone()))
            .filter(bookmark::Column::Blockchain.eq(network_id.blockchain_type()))
            .one(&self.conn)
            .await?;

        if let Some(model) = existing {
            let mut update_model: bookmark::ActiveModel = model.into();
            update_model.height = Set(new_height as i32);
            update_model.status = Set("confirmed".to_string());
            update_model.last_updated_at = Set(Utc::now().into());
            update_model.update(&self.conn).await?;
            Ok(())
        } else {
            // No existing bookmark, create one with placeholder hash
            let bookmark = bookmark::ActiveModel {
                hash: Set(format!("fast_reindex_{}", new_height)),
                height: Set(new_height as i32),
                status: Set("confirmed".to_string()),
                last_updated_at: Set(Utc::now().into()),
                network: Set(network_id.name.clone()),
                blockchain: Set(network_id.blockchain_type()),
            };
            bookmark.insert(&self.conn).await?;
            Ok(())
        }
    }
}
