use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter};
use std::collections::{HashMap, HashSet};

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

    /// Batch get likes counts for multiple charms (single query)
    pub async fn get_likes_counts_batch(
        &self,
        charm_ids: &[String],
    ) -> Result<HashMap<String, i64>, DbErr> {
        if charm_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Get all likes for these charm_ids in one query
        let likes = Likes::find()
            .filter(likes::Column::CharmId.is_in(charm_ids.to_vec()))
            .all(&self.db)
            .await?;

        // Count likes per charm_id
        let mut counts: HashMap<String, i64> = HashMap::new();
        for like in likes {
            *counts.entry(like.charm_id).or_insert(0) += 1;
        }

        Ok(counts)
    }

    /// Batch check if user liked multiple charms (single query)
    pub async fn get_user_likes_batch(
        &self,
        charm_ids: &[String],
        user_id: i32,
    ) -> Result<HashSet<String>, DbErr> {
        if charm_ids.is_empty() {
            return Ok(HashSet::new());
        }

        // Get all likes by this user for these charm_ids in one query
        let likes = Likes::find()
            .filter(likes::Column::CharmId.is_in(charm_ids.to_vec()))
            .filter(likes::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;

        // Collect charm_ids that user has liked
        let liked_set: HashSet<String> = likes.into_iter().map(|l| l.charm_id).collect();

        Ok(liked_set)
    }
}
