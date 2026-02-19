use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::fmt;

use crate::infrastructure::persistence::error::DbError;

/// Repository for address_utxos table operations
/// Handles inserting new UTXOs and deleting spent ones during block processing
#[derive(Clone)]
pub struct UtxoRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for UtxoRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UtxoRepository").finish_non_exhaustive()
    }
}

/// A single UTXO to be inserted
#[derive(Debug, Clone)]
pub struct UtxoInsert {
    pub txid: String,
    pub vout: i32,
    pub address: String,
    pub value: i64,
    pub script_pubkey: String,
    pub block_height: i32,
    pub network: String,
}

impl UtxoRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert a batch of new UTXOs (from block outputs)
    /// Uses ON CONFLICT DO NOTHING to handle duplicates gracefully
    pub async fn insert_batch(&self, utxos: &[UtxoInsert]) -> Result<usize, DbError> {
        if utxos.is_empty() {
            return Ok(0);
        }

        // Build batch INSERT with VALUES list, chunked to avoid huge queries
        let mut total_inserted = 0usize;
        for chunk in utxos.chunks(500) {
            let values: Vec<String> = chunk
                .iter()
                .map(|u| {
                    format!(
                        "('{}', {}, '{}', '{}', {}, '{}', {})",
                        u.txid.replace('\'', "''"),
                        u.vout,
                        u.network.replace('\'', "''"),
                        u.address.replace('\'', "''"),
                        u.value,
                        u.script_pubkey.replace('\'', "''"),
                        u.block_height,
                    )
                })
                .collect();

            let sql = format!(
                "INSERT INTO address_utxos (txid, vout, network, address, value, script_pubkey, block_height) VALUES {} ON CONFLICT (txid, vout, network) DO NOTHING",
                values.join(", ")
            );

            let result = self
                .conn
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

            total_inserted += result.rows_affected() as usize;
        }

        Ok(total_inserted)
    }

    /// Delete spent UTXOs (from block inputs)
    /// Each item is (txid, vout) of the previous output being spent
    pub async fn delete_spent_batch(
        &self,
        spent: &[(String, i32)],
        network: &str,
    ) -> Result<usize, DbError> {
        if spent.is_empty() {
            return Ok(0);
        }

        let mut total_deleted = 0usize;
        for chunk in spent.chunks(500) {
            let conditions: Vec<String> = chunk
                .iter()
                .map(|(txid, vout)| {
                    format!("(txid = '{}' AND vout = {})", txid.replace('\'', "''"), vout)
                })
                .collect();

            let sql = format!(
                "DELETE FROM address_utxos WHERE network = '{}' AND ({})",
                network.replace('\'', "''"),
                conditions.join(" OR ")
            );

            let result = self
                .conn
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

            total_deleted += result.rows_affected() as usize;
        }

        Ok(total_deleted)
    }

    /// Delete all UTXOs for a given block height and network (for rollback/reindex)
    pub async fn delete_by_block(
        &self,
        block_height: i32,
        network: &str,
    ) -> Result<usize, DbError> {
        let sql = format!(
            "DELETE FROM address_utxos WHERE block_height = {} AND network = '{}'",
            block_height,
            network.replace('\'', "''")
        );

        let result = self
            .conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    /// Get UTXO count for monitoring
    pub async fn count(&self, network: &str) -> Result<i64, DbError> {
        let sql = format!(
            "SELECT COUNT(*) as cnt FROM address_utxos WHERE network = '{}'",
            network.replace('\'', "''")
        );

        let result = self
            .conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        match result {
            Some(row) => {
                let count: i64 = row
                    .try_get("", "cnt")
                    .map_err(|e| DbError::QueryError(e.to_string()))?;
                Ok(count)
            }
            None => Ok(0),
        }
    }
}
