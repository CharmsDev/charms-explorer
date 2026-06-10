use std::collections::HashSet;
use std::fmt;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

use crate::infrastructure::persistence::error::DbError;

/// Repository for monitored_addresses table operations.
/// Tracks which addresses the indexer should maintain UTXO data for.
#[derive(Clone)]
pub struct MonitoredAddressesRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for MonitoredAddressesRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitoredAddressesRepository")
            .finish_non_exhaustive()
    }
}

impl MonitoredAddressesRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Load all monitored addresses for a network into a HashSet (for fast lookup).
    pub async fn load_set(&self, network: &str) -> Result<HashSet<String>, DbError> {
        let sql = format!(
            "SELECT address FROM monitored_addresses WHERE network = '{}'",
            network.replace('\'', "''")
        );

        let rows = self
            .conn
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut set = HashSet::with_capacity(rows.len());
        for row in rows {
            if let Ok(addr) = row.try_get::<String>("", "address") {
                set.insert(addr);
            }
        }

        Ok(set)
    }

    /// Register a batch of addresses (from charm detection in a single block).
    pub async fn register_batch(
        &self,
        addresses: &[String],
        network: &str,
        source: &str,
    ) -> Result<usize, DbError> {
        if addresses.is_empty() {
            return Ok(0);
        }

        let mut total = 0usize;
        for chunk in addresses.chunks(500) {
            let values: Vec<String> = chunk
                .iter()
                .map(|addr| {
                    format!(
                        "('{}', '{}', '{}', NOW())",
                        addr.replace('\'', "''"),
                        network.replace('\'', "''"),
                        source.replace('\'', "''"),
                    )
                })
                .collect();

            let sql = format!(
                "INSERT INTO monitored_addresses (address, network, source, created_at) \
                 VALUES {} ON CONFLICT (address, network) DO NOTHING",
                values.join(", ")
            );

            let result = self
                .conn
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

            total += result.rows_affected() as usize;
        }

        Ok(total)
    }

    /// Load only seeded addresses (seeded_at IS NOT NULL) for a network into a HashSet.
    /// These are addresses whose BTC UTXOs have been populated and should be tracked in real time.
    pub async fn load_seeded_set(&self, network: &str) -> Result<HashSet<String>, DbError> {
        let sql = format!(
            "SELECT address FROM monitored_addresses WHERE network = '{}' AND seeded_at IS NOT NULL",
            network.replace('\'', "''")
        );

        let rows = self
            .conn
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut set = HashSet::with_capacity(rows.len());
        for row in rows {
            if let Ok(addr) = row.try_get::<String>("", "address") {
                set.insert(addr);
            }
        }

        Ok(set)
    }

}
