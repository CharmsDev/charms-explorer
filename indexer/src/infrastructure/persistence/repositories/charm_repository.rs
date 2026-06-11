use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    Statement,
};

use crate::infrastructure::persistence::entities::charms;
use crate::infrastructure::persistence::error::DbError;

/// Repository for charm operations
#[derive(Clone, Debug)]
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Create a new CharmRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Optimize the session for high-throughput writer by toggling synchronous_commit
    pub async fn set_synchronous_commit(&self, on: bool) -> Result<(), DbError> {
        let value = if on { "on" } else { "off" };
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!("SET synchronous_commit = {};", value),
        );
        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Save a charm
    /// Get distinct block heights that have charms for a network
    pub async fn get_distinct_block_heights(&self, network: &str) -> Result<Vec<u64>, DbError> {
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT DISTINCT block_height FROM charms WHERE network = '{}' AND block_height IS NOT NULL ORDER BY block_height",
                network
            ),
        );

        let results = self
            .conn
            .query_all(stmt)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let heights: Vec<u64> = results
            .iter()
            .filter_map(|row| {
                #[allow(unused_imports)]
                use sea_orm::TryGetable;
                let height: Option<i32> = row.try_get("", "block_height").ok();
                height.map(|h| h as u64)
            })
            .collect();

        Ok(heights)
    }

    /// Save multiple charms in a batch
    /// Updated signature: removed charmid, added vout, app_id, and amount
    /// Added address field
    /// Added tags field
    pub async fn save_batch(
        &self,
        charms: Vec<(
            String,            // txid
            i32,               // vout
            u64,               // block_height
            serde_json::Value, // data
            String,            // asset_type
            String,            // blockchain
            String,            // network
            Option<String>,    // address
            String,            // app_id
            i64,               // amount
            Option<String>,    // tags
        )>,
    ) -> Result<Vec<(String, i32)>, DbError> {
        if charms.is_empty() {
            return Ok(vec![]);
        }

        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        // Build raw SQL with ON CONFLICT DO NOTHING so that duplicates are
        // silently skipped while the rest of the batch is still inserted.
        // Returns the (txid, vout) pairs that were actually inserted so callers
        // can update stats_holders only for truly new charms (not mempool-promoted ones).
        let mut values_parts: Vec<String> = Vec::with_capacity(charms.len());

        for (txid, vout, block_height, data, asset_type, blockchain, network, address, app_id, amount, tags) in &charms {
            let addr_sql = match address {
                Some(a) => format!("'{}'", a.replace('\'', "''")),
                None => "NULL".to_string(),
            };
            let tags_sql = match tags {
                Some(t) => format!("'{}'", t.replace('\'', "''")),
                None => "NULL".to_string(),
            };
            let data_json = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());

            values_parts.push(format!(
                "('{}', {}, {}, '{}'::jsonb, '{}', '{}', '{}', '{}', {}, false, '{}', {}, NULL, {}, true)",
                txid.replace('\'', "''"),
                vout,
                block_height,
                data_json.replace('\'', "''"),
                now_str,
                asset_type.replace('\'', "''"),
                blockchain.replace('\'', "''"),
                network.replace('\'', "''"),
                addr_sql,
                app_id.replace('\'', "''"),
                amount,
                tags_sql,
            ));
        }

        let sql = format!(
            "INSERT INTO charms (txid, vout, block_height, data, date_created, asset_type, blockchain, network, address, spent, app_id, amount, mempool_detected_at, tags, verified) \
             VALUES {} \
             ON CONFLICT (txid, vout) DO NOTHING \
             RETURNING txid, vout",
            values_parts.join(", ")
        );

        let rows = self.conn
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let inserted: Vec<(String, i32)> = rows
            .iter()
            .filter_map(|row| {
                let txid: String = row.try_get("", "txid").ok()?;
                let vout: i32 = row.try_get("", "vout").ok()?;
                Some((txid, vout))
            })
            .collect();

        Ok(inserted)
    }

    /// Mark multiple charms as spent in a batch using (txid, vout) pairs.
    /// Scoped by `network` so collisions across mainnet/testnet do not bleed
    /// into each other.
    pub async fn mark_charms_as_spent_batch(
        &self,
        txid_vouts: Vec<(String, i32)>,
        network: &str,
    ) -> Result<(), DbError> {
        if txid_vouts.is_empty() {
            return Ok(());
        }

        let values = txid_vouts
            .iter()
            .map(|(txid, vout)| format!("('{}', {})", txid.replace('\'', "''"), vout))
            .collect::<Vec<_>>()
            .join(", ");

        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "UPDATE charms SET spent = true \
                 WHERE (txid, vout) IN (VALUES {}) AND spent = false AND network = '{}'",
                values,
                network.replace('\'', "''"),
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Get charm info for stats_holders updates before marking as spent.
    /// Returns (app_id, address, amount). Scoped by `network`.
    pub async fn get_charms_for_spent_update(
        &self,
        txid_vouts: Vec<(String, i32)>,
        network: &str,
    ) -> Result<Vec<(String, String, i64)>, DbError> {
        if txid_vouts.is_empty() {
            return Ok(vec![]);
        }

        let values = txid_vouts
            .iter()
            .map(|(txid, vout)| format!("('{}', {})", txid.replace('\'', "''"), vout))
            .collect::<Vec<_>>()
            .join(", ");

        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT app_id, address, amount FROM charms \
                 WHERE (txid, vout) IN (VALUES {}) \
                 AND spent = false AND address IS NOT NULL AND network = '{}'",
                values,
                network.replace('\'', "''"),
            ),
        );

        let results = self.conn.query_all(stmt).await?;

        Ok(results
            .into_iter()
            .filter_map(|row| {
                let app_id: String = row.try_get("", "app_id").ok()?;
                let address: String = row.try_get("", "address").ok()?;
                let amount: i64 = row.try_get("", "amount").ok()?;
                Some((app_id, address, amount))
            })
            .collect())
    }

    /// Get amounts by txids for calculating net supply change
    /// Returns: Vec<(txid, app_id, amount)>
    pub async fn get_amounts_by_txids(
        &self,
        txids: &[String],
    ) -> Result<Vec<(String, String, u64)>, DbError> {
        if txids.is_empty() {
            return Ok(vec![]);
        }

        let results = charms::Entity::find()
            .filter(charms::Column::Txid.is_in(txids.iter().map(|s| s.as_str())))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|c| (c.txid, c.app_id, c.amount as u64))
            .collect())
    }

    /// Get unspent charms by (txid, vout) pairs
    /// Returns (txid, vout, app_id, address, amount) for unspent charms only
    pub async fn get_unspent_charms_by_txid_vout(
        &self,
        txid_vouts: Vec<(String, i32)>,
    ) -> Result<Vec<(String, i32, String, Option<String>, i64)>, DbError> {
        if txid_vouts.is_empty() {
            return Ok(vec![]);
        }

        // Build OR conditions for each (txid, vout) pair
        let mut conditions = sea_orm::Condition::any();
        for (txid, vout) in &txid_vouts {
            conditions = conditions.add(
                sea_orm::Condition::all()
                    .add(charms::Column::Txid.eq(txid.clone()))
                    .add(charms::Column::Vout.eq(*vout)),
            );
        }

        let results = charms::Entity::find()
            .filter(conditions)
            .filter(charms::Column::Spent.eq(false))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|c| (c.txid, c.vout, c.app_id, c.address, c.amount))
            .collect())
    }

    /// Check if any of the given txids belongs to a known beam-out transaction.
    /// Used as a heuristic to detect ADA→BTC claims: when a user claims tokens back from
    /// Cardano, they often fund the claim tx with outputs from their prior BTC→ADA beam-out.
    /// If a spell that creates tokens has an input from a beam-out tx, it's likely a claim.
    pub async fn has_beam_out_input_txid(&self, input_txids: &[String]) -> Result<bool, DbError> {
        if input_txids.is_empty() {
            return Ok(false);
        }
        // txids are 64-char hex — safe to inline
        let txid_list = input_txids
            .iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT EXISTS (SELECT 1 FROM charms WHERE txid IN ({}) AND tags LIKE '%beam-out%') AS result",
            txid_list
        );
        let stmt = Statement::from_string(DbBackend::Postgres, sql);
        let result = self.conn.query_one(stmt).await?;
        Ok(result
            .and_then(|r| r.try_get::<bool>("", "result").ok())
            .unwrap_or(false))
    }

    /// Get unspent charms by block height and network
    /// Returns (app_id, address, amount) for stats_holders updates during reindex
    pub async fn get_unspent_charms_by_block(
        &self,
        block_height: i32,
        network: &str,
    ) -> Result<Vec<(String, Option<String>, i64)>, DbError> {
        let results = charms::Entity::find()
            .filter(charms::Column::BlockHeight.eq(block_height))
            .filter(charms::Column::Network.eq(network))
            .filter(charms::Column::Spent.eq(false))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|c| (c.app_id, c.address, c.amount))
            .collect())
    }
}
