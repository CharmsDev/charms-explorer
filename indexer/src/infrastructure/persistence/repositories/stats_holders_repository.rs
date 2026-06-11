// Repository for stats_holders table operations in indexer

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

use crate::infrastructure::persistence::error::DbError;

/// Repository for holder statistics operations.
///
/// All public methods take a `network` argument so mainnet and testnet4
/// balances do not collide (audit finding N2).
#[derive(Clone, Debug)]
pub struct StatsHoldersRepository {
    conn: DatabaseConnection,
}

impl StatsHoldersRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// UPSERT a single holder's running balance. Adds `amount_delta` (signed)
    /// to the existing total or inserts a new row keyed by
    /// `(app_id, address, network)`. Negative deltas also trigger a cleanup
    /// of zero-balance rows so the holder set stays accurate.
    pub async fn update_holder_stats(
        &self,
        app_id: &str,
        address: &str,
        network: &str,
        amount_delta: i64,
        block_height: i32,
    ) -> Result<(), DbError> {
        // Cap amount_delta to prevent bigint overflow on extreme inputs.
        let capped_amount = if amount_delta > 0 {
            amount_delta.min(i64::MAX / 2)
        } else {
            amount_delta.max(i64::MIN / 2)
        };

        // Idempotency gate (audit N1): the WHERE on the DO UPDATE skips the
        // increment when `last_updated_block` already matches or exceeds the
        // current block, so re-processing the same block after a crash will
        // not double-count balances. New rows always insert normally.
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                r#"
                INSERT INTO stats_holders
                    (app_id, address, network, total_amount, charm_count, first_seen_block, last_updated_block, created_at, updated_at)
                VALUES
                    ('{app_id}', '{address}', '{network}', {amount}, 1, {block}, {block}, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                ON CONFLICT (app_id, address, network)
                DO UPDATE SET
                    total_amount = stats_holders.total_amount + {amount},
                    charm_count = CASE
                        WHEN {amount} > 0 THEN stats_holders.charm_count + 1
                        ELSE stats_holders.charm_count - 1
                    END,
                    last_updated_block = {block},
                    updated_at = CURRENT_TIMESTAMP
                WHERE stats_holders.last_updated_block < {block}
                "#,
                app_id = app_id.replace('\'', "''"),
                address = address.replace('\'', "''"),
                network = network.replace('\'', "''"),
                amount = capped_amount,
                block = block_height,
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        if amount_delta < 0 {
            self.cleanup_zero_holders(app_id, address, network).await?;
        }

        Ok(())
    }

    /// Remove the (app_id, address, network) row if its balance dropped to zero.
    async fn cleanup_zero_holders(
        &self,
        app_id: &str,
        address: &str,
        network: &str,
    ) -> Result<(), DbError> {
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "DELETE FROM stats_holders WHERE app_id = '{}' AND address = '{}' AND network = '{}' AND total_amount <= 0",
                app_id.replace('\'', "''"),
                address.replace('\'', "''"),
                network.replace('\'', "''"),
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Group updates by (app_id, address) and apply them per network.
    /// The grouping reduces the number of UPSERTs when the same holder
    /// receives several charms in the same block.
    pub async fn update_holders_batch(
        &self,
        updates: Vec<(String, String, i64, i32)>,
        network: &str,
    ) -> Result<(), DbError> {
        if updates.is_empty() {
            return Ok(());
        }

        use std::collections::HashMap;
        let mut grouped: HashMap<(String, String), (i64, i32)> = HashMap::new();

        for (app_id, address, amount, block_height) in updates {
            let key = (app_id.clone(), address.clone());
            let entry = grouped.entry(key).or_insert((0, block_height));
            let old_value = entry.0;
            entry.0 = entry.0.checked_add(amount).unwrap_or_else(|| {
                crate::utils::logging::log_warning(&format!(
                    "[STATS_HOLDERS] Overflow adding {} to {} for {}/{}",
                    amount, old_value, app_id, address
                ));
                old_value
            });
            entry.1 = entry.1.max(block_height);
        }

        for ((app_id, address), (total_delta, block_height)) in grouped {
            self.update_holder_stats(&app_id, &address, network, total_delta, block_height)
                .await?;
        }

        Ok(())
    }
}
