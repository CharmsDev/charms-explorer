use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, PaginatorTrait,
};

use crate::entity::{likes, prelude::Likes};

/// Repository for managing likes in the database
pub struct LikesRepository {
    db: DatabaseConnection,
}

impl LikesRepository {
    /// Creates a new likes repository with the given database connection
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Adds a like for a charm by a user
    pub async fn add_like(&self, charm_id: &str, user_id: i32) -> Result<i64, DbErr> {
        // Check if the like already exists
        let existing_like = Likes::find()
            .filter(likes::Column::CharmId.eq(charm_id))
            .filter(likes::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?;

        // If the like already exists, return the count
        if existing_like.is_some() {
            return self.get_likes_count(charm_id).await;
        }

        // Create a new like
        let like = likes::ActiveModel {
            charm_id: sea_orm::ActiveValue::Set(charm_id.to_string()),
            user_id: sea_orm::ActiveValue::Set(user_id),
            ..Default::default()
        };

        // Insert the like
        Likes::insert(like).exec(&self.db).await?;

        // Return the updated count
        self.get_likes_count(charm_id).await
    }

    /// Removes a like for a charm by a user
    pub async fn remove_like(&self, charm_id: &str, user_id: i32) -> Result<i64, DbErr> {
        // Delete the like
        Likes::delete_many()
            .filter(likes::Column::CharmId.eq(charm_id))
            .filter(likes::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        // Return the updated count
        self.get_likes_count(charm_id).await
    }

    /// Counts the number of likes for a charm
    pub async fn get_likes_count(&self, charm_id: &str) -> Result<i64, DbErr> {
        let count = Likes::find()
            .filter(likes::Column::CharmId.eq(charm_id))
            .count(&self.db)
            .await?;
        
        Ok(count as i64)
    }

    /// Checks if a user has liked a charm
    pub async fn has_user_liked(&self, charm_id: &str, user_id: i32) -> Result<bool, DbErr> {
        let count = Likes::find()
            .filter(likes::Column::CharmId.eq(charm_id))
            .filter(likes::Column::UserId.eq(user_id))
            .count(&self.db)
            .await?;

        Ok(count > 0)
    }

    /// Gets all likes for a list of charms
    pub async fn get_likes_for_charms(
        &self,
        charm_ids: &[String],
        user_id: Option<i32>,
    ) -> Result<Vec<(String, i64, bool)>, DbErr> {
        let mut result = Vec::new();

        // For each charm ID, get the like count and whether the user has liked it
        for charm_id in charm_ids {
            let count = self.get_likes_count(charm_id).await?;
            let user_liked = match user_id {
                Some(uid) => self.has_user_liked(charm_id, uid).await?,
                None => false,
            };
            result.push((charm_id.clone(), count, user_liked));
        }

        Ok(result)
    }
}
