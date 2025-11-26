// Repository for stats_holders table operations
use crate::db::DbError;
use crate::entity::stats_holders;
use sea_orm::*;

pub struct StatsHoldersRepository {
    conn: DatabaseConnection,
}

impl StatsHoldersRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get holders for a specific app_id, ordered by total_amount DESC
    pub async fn get_holders_by_app_id(
        &self,
        app_id: &str,
    ) -> Result<Vec<stats_holders::Model>, DbError> {
        stats_holders::Entity::find()
            .filter(stats_holders::Column::AppId.eq(app_id))
            .order_by_desc(stats_holders::Column::TotalAmount)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Get holder info for a specific (app_id, address) combination
    pub async fn get_holder(
        &self,
        app_id: &str,
        address: &str,
    ) -> Result<Option<stats_holders::Model>, DbError> {
        stats_holders::Entity::find()
            .filter(stats_holders::Column::AppId.eq(app_id))
            .filter(stats_holders::Column::Address.eq(address))
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Upsert holder statistics (insert or update)
    pub async fn upsert_holder(
        &self,
        app_id: &str,
        address: &str,
        total_amount: i64,
        charm_count: i32,
        block_height: i32,
    ) -> Result<(), DbError> {
        // Try to find existing record
        let existing = self.get_holder(app_id, address).await?;

        if let Some(holder) = existing {
            // Update existing record
            let mut holder: stats_holders::ActiveModel = holder.into();
            holder.total_amount = Set(total_amount);
            holder.charm_count = Set(charm_count);
            holder.last_updated_block = Set(block_height);
            holder.updated_at = Set(chrono::Local::now().naive_local());
            holder.update(&self.conn).await?;
        } else {
            // Insert new record
            let new_holder = stats_holders::ActiveModel {
                app_id: Set(app_id.to_string()),
                address: Set(address.to_string()),
                total_amount: Set(total_amount),
                charm_count: Set(charm_count),
                first_seen_block: Set(block_height),
                last_updated_block: Set(block_height),
                created_at: Set(chrono::Local::now().naive_local()),
                updated_at: Set(chrono::Local::now().naive_local()),
                ..Default::default()
            };
            new_holder.insert(&self.conn).await?;
        }

        Ok(())
    }

    /// Delete holder record (when total_amount reaches 0)
    pub async fn delete_holder(
        &self,
        app_id: &str,
        address: &str,
    ) -> Result<(), DbError> {
        stats_holders::Entity::delete_many()
            .filter(stats_holders::Column::AppId.eq(app_id))
            .filter(stats_holders::Column::Address.eq(address))
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    /// Get total holder count for an app_id
    pub async fn get_holder_count(&self, app_id: &str) -> Result<u64, DbError> {
        let count = stats_holders::Entity::find()
            .filter(stats_holders::Column::AppId.eq(app_id))
            .count(&self.conn)
            .await?;
        Ok(count)
    }
}
