use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};

use crate::infrastructure::persistence::entities::bookmark;
use crate::infrastructure::persistence::error::DbError;

/// Repository for bookmark operations
#[derive(Clone)]
pub struct BookmarkRepository {
    conn: DatabaseConnection,
}

impl BookmarkRepository {
    /// Create a new BookmarkRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get the last processed block
    pub async fn get_last_processed_block(&self) -> Result<Option<u64>, DbError> {
        // Query the database for the last processed block
        let result = bookmark::Entity::find()
            .order_by_desc(bookmark::Column::Height)
            .one(&self.conn)
            .await?;

        // Return the height if found
        Ok(result.map(|b| b.height as u64))
    }

    /// Save a bookmark
    pub async fn save_bookmark(
        &self,
        hash: &str,
        height: u64,
        is_confirmed: bool,
    ) -> Result<(), DbError> {
        // Create a new active model
        let status = if is_confirmed {
            "confirmed".to_string()
        } else {
            "pending".to_string()
        };

        let bookmark = bookmark::ActiveModel {
            hash: Set(hash.to_string()),
            height: Set(height as i32),
            status: Set(status),
            last_updated_at: Set(Utc::now().into()),
        };

        // Insert or update the bookmark
        bookmark.insert(&self.conn).await?;

        Ok(())
    }

    /// Get the last updated timestamp
    pub async fn get_last_updated_timestamp(&self) -> Result<Option<DateTime<Utc>>, DbError> {
        // Query the database for the last updated bookmark
        let result = bookmark::Entity::find()
            .order_by_desc(bookmark::Column::LastUpdatedAt)
            .one(&self.conn)
            .await?;

        // Return the timestamp if found
        Ok(result.map(|b| b.last_updated_at.into()))
    }
}
