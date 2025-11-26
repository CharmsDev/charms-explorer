// [RJJ-STATS-HOLDERS] Repository for stats_holders table operations in indexer

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    ConnectionTrait, Statement, DbBackend,
};

use crate::infrastructure::persistence::entities::stats_holders;
use crate::infrastructure::persistence::error::DbError;

/// Repository for holder statistics operations
#[derive(Clone, Debug)]
pub struct StatsHoldersRepository {
    conn: DatabaseConnection,
}

impl StatsHoldersRepository {
    /// Create a new StatsHoldersRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Update holder statistics for a specific (app_id, address) combination
    /// This is called when a new charm is created or when a charm is spent
    pub async fn update_holder_stats(
        &self,
        app_id: &str,
        address: &str,
        amount_delta: i64, // Positive for new charm, negative for spent charm
        block_height: i32,
    ) -> Result<(), DbError> {
        // Use raw SQL for efficient UPSERT with amount delta
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                r#"
                INSERT INTO stats_holders 
                    (app_id, address, total_amount, charm_count, first_seen_block, last_updated_block, created_at, updated_at)
                VALUES 
                    ('{}', '{}', {}, 1, {}, {}, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                ON CONFLICT (app_id, address) 
                DO UPDATE SET
                    total_amount = stats_holders.total_amount + {},
                    charm_count = CASE 
                        WHEN {} > 0 THEN stats_holders.charm_count + 1
                        ELSE stats_holders.charm_count - 1
                    END,
                    last_updated_block = {},
                    updated_at = CURRENT_TIMESTAMP
                "#,
                app_id.replace("'", "''"),
                address.replace("'", "''"),
                amount_delta,
                block_height,
                block_height,
                amount_delta,
                amount_delta,
                block_height
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        // Clean up holders with zero balance
        if amount_delta < 0 {
            self.cleanup_zero_holders(app_id, address).await?;
        }

        Ok(())
    }

    /// Remove holder records with zero or negative balance
    async fn cleanup_zero_holders(&self, app_id: &str, address: &str) -> Result<(), DbError> {
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "DELETE FROM stats_holders WHERE app_id = '{}' AND address = '{}' AND total_amount <= 0",
                app_id.replace("'", "''"),
                address.replace("'", "''")
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Batch update holder statistics for multiple charms
    pub async fn update_holders_batch(
        &self,
        updates: Vec<(String, String, i64, i32)>, // (app_id, address, amount_delta, block_height)
    ) -> Result<(), DbError> {
        if updates.is_empty() {
            return Ok(());
        }

        // Group by (app_id, address) and sum amounts
        use std::collections::HashMap;
        let mut grouped: HashMap<(String, String), (i64, i32)> = HashMap::new();
        
        for (app_id, address, amount, block_height) in updates {
            let key = (app_id, address);
            let entry = grouped.entry(key).or_insert((0, block_height));
            entry.0 += amount;
            entry.1 = entry.1.max(block_height);
        }

        // Apply each grouped update
        for ((app_id, address), (total_delta, block_height)) in grouped {
            self.update_holder_stats(&app_id, &address, total_delta, block_height).await?;
        }

        Ok(())
    }
}
