//! Repository for mempool_spends table
//! Tracks which UTXOs are being spent by unconfirmed mempool transactions.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement,
};

use crate::infrastructure::persistence::entities::mempool_spends;
use crate::infrastructure::persistence::error::DbError;

#[derive(Clone, Debug)]
pub struct MempoolSpendsRepository {
    conn: DatabaseConnection,
}

impl MempoolSpendsRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Expose the underlying connection (needed by MempoolProcessor for direct entity inserts)
    pub fn get_connection(&self) -> DatabaseConnection {
        self.conn.clone()
    }

    /// Record that a mempool tx is spending a UTXO.
    /// Uses ON CONFLICT DO NOTHING — safe to call multiple times.
    pub async fn record_spend(
        &self,
        spending_txid: &str,
        spent_txid: &str,
        spent_vout: i32,
        network: &str,
    ) -> Result<(), DbError> {
        let model = mempool_spends::ActiveModel {
            spending_txid: Set(spending_txid.to_string()),
            spent_txid: Set(spent_txid.to_string()),
            spent_vout: Set(spent_vout),
            network: Set(network.to_string()),
            detected_at: Set(Utc::now()),
        };

        match model.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("duplicate key") {
                    Ok(())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Record multiple spends in a single batch INSERT.
    /// Each item: (spending_txid, spent_txid, spent_vout)
    pub async fn record_spends_batch(
        &self,
        spends: &[(String, String, i32)],
        network: &str,
    ) -> Result<(), DbError> {
        if spends.is_empty() {
            return Ok(());
        }

        let now = Utc::now();
        let values: Vec<String> = spends
            .iter()
            .map(|(spending, spent_txid, spent_vout)| {
                format!(
                    "('{}', '{}', {}, '{}', '{}')",
                    spending.replace('\'', "''"),
                    spent_txid.replace('\'', "''"),
                    spent_vout,
                    network,
                    now.to_rfc3339(),
                )
            })
            .collect();

        let sql = format!(
            "INSERT INTO mempool_spends (spending_txid, spent_txid, spent_vout, network, detected_at) \
             VALUES {} ON CONFLICT (spent_txid, spent_vout, network) DO NOTHING",
            values.join(", ")
        );

        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Remove mempool spend records when a tx gets confirmed in a block.
    /// Called by block_processor when processing a new block.
    pub async fn remove_confirmed_spends(
        &self,
        spending_txids: &[String],
        network: &str,
    ) -> Result<(), DbError> {
        if spending_txids.is_empty() {
            return Ok(());
        }

        let ids: Vec<String> = spending_txids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();

        let sql = format!(
            "DELETE FROM mempool_spends WHERE spending_txid IN ({}) AND network = '{}'",
            ids.join(", "),
            network
        );

        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Remove all mempool spend records for a specific spending tx (e.g. RBF eviction).
    pub async fn remove_by_spending_txid(
        &self,
        spending_txid: &str,
        network: &str,
    ) -> Result<(), DbError> {
        mempool_spends::Entity::delete_many()
            .filter(mempool_spends::Column::SpendingTxid.eq(spending_txid))
            .filter(mempool_spends::Column::Network.eq(network))
            .exec(&self.conn)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Purge stale mempool spends older than `max_age_hours`.
    /// Called periodically to clean up txs that were never confirmed (expired/RBF).
    pub async fn purge_stale(&self, max_age_hours: i64) -> Result<u64, DbError> {
        let sql = format!(
            "DELETE FROM mempool_spends WHERE detected_at < NOW() - INTERVAL '{} hours'",
            max_age_hours
        );

        let result = self
            .conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Get all UTXOs currently being spent in mempool for a network.
    /// Returns (spent_txid, spent_vout) pairs.
    pub async fn get_mempool_spent_utxos(
        &self,
        network: &str,
    ) -> Result<Vec<(String, i32)>, DbError> {
        let results = mempool_spends::Entity::find()
            .filter(mempool_spends::Column::Network.eq(network))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|m| (m.spent_txid, m.spent_vout))
            .collect())
    }
}
