//! Repository for mempool_spends table
//! Tracks which UTXOs are being spent by unconfirmed mempool transactions.

use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    Statement,
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

        // Bind every value instead of interpolating. Txids reach us from RPC
        // and gateway JSON, so keeping them out of the SQL string removes the
        // injection surface entirely rather than relying on manual quoting.
        let mut params: Vec<sea_orm::Value> = Vec::with_capacity(spends.len() * 5);
        let mut tuples: Vec<String> = Vec::with_capacity(spends.len());
        for (i, (spending, spent_txid, spent_vout)) in spends.iter().enumerate() {
            let b = i * 5;
            tuples.push(format!(
                "(${}, ${}, ${}, ${}, ${})",
                b + 1,
                b + 2,
                b + 3,
                b + 4,
                b + 5
            ));
            params.push(spending.as_str().into());
            params.push(spent_txid.as_str().into());
            params.push((*spent_vout).into());
            params.push(network.into());
            params.push(now.into());
        }

        // Last-spender-wins: under RBF / double-spend in the mempool the
        // newest spending tx is what the API should report as the pending
        // consumer of the UTXO. DO NOTHING would freeze the first observed
        // spender even after it is dropped, leaving stale rows once the
        // reconcile loop cleans the original tx.
        let sql = format!(
            "INSERT INTO mempool_spends (spending_txid, spent_txid, spent_vout, network, detected_at) \
             VALUES {} \
             ON CONFLICT (spent_txid, spent_vout, network) DO UPDATE SET \
               spending_txid = EXCLUDED.spending_txid, \
               detected_at = EXCLUDED.detected_at",
            tuples.join(", ")
        );

        self.conn
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                params,
            ))
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

        let placeholders: Vec<String> = (1..=spending_txids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            "DELETE FROM mempool_spends WHERE spending_txid IN ({}) AND network = ${}",
            placeholders.join(", "),
            spending_txids.len() + 1
        );

        let mut params: Vec<sea_orm::Value> =
            spending_txids.iter().map(|id| id.as_str().into()).collect();
        params.push(network.into());

        self.conn
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                params,
            ))
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

    /// Purge stale mempool spends older than `max_age_hours` for one network.
    /// Called periodically to clean up txs that were never confirmed (expired/RBF).
    ///
    /// The network filter matters: one MempoolProcessor runs per network and
    /// each calls this on its own cleanup cycle. Without the filter the mainnet
    /// processor also deleted testnet4 rows (and vice versa), breaking the
    /// per-network isolation the rest of the persistence layer maintains and
    /// mis-attributing the purge count in the logs.
    pub async fn purge_stale(&self, network: &str, max_age_hours: i64) -> Result<u64, DbError> {
        let result = self
            .conn
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM mempool_spends \
                 WHERE network = $1 AND detected_at < NOW() - make_interval(hours => $2)",
                [network.into(), (max_age_hours as i32).into()],
            ))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(result.rows_affected())
    }

}
