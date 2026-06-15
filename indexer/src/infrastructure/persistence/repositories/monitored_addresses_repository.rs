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

    /// Fetch a batch of addresses pending a Maestro seed (seeded_at IS NULL).
    /// Used by the BTC auto-seeder worker. Oldest registrations first so
    /// addresses don't starve when the queue is permanently busy.
    pub async fn fetch_unseeded(
        &self,
        network: &str,
        limit: u64,
    ) -> Result<Vec<String>, DbError> {
        let sql = format!(
            "SELECT address FROM monitored_addresses \
             WHERE network = '{}' AND seeded_at IS NULL \
             ORDER BY created_at ASC LIMIT {}",
            network.replace('\'', "''"),
            limit,
        );
        let rows = self
            .conn
            .query_all(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<String>("", "address").ok())
            .collect())
    }

    /// Mark an address as seeded by persisting the Maestro tip cursor.
    /// The block_hash + height pair is what `api::is_seeded` later validates
    /// against `block_status` to detect reorgs between seed and handoff.
    pub async fn mark_seeded(
        &self,
        address: &str,
        network: &str,
        seed_height: i32,
        seed_block_hash: &str,
    ) -> Result<(), DbError> {
        let sql = format!(
            "UPDATE monitored_addresses \
             SET seeded_at = NOW(), seed_height = {}, seed_block_hash = '{}' \
             WHERE address = '{}' AND network = '{}'",
            seed_height,
            seed_block_hash.replace('\'', "''"),
            address.replace('\'', "''"),
            network.replace('\'', "''"),
        );
        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Try-acquire a PG advisory lock keyed by (address, network). Returns
    /// true if acquired (the seeder owns the address for this run), false
    /// if another worker (API on-demand seeder, this worker on another node,
    /// or a previous run that died without releasing) already holds it.
    pub async fn try_advisory_lock(
        &self,
        address: &str,
        network: &str,
    ) -> Result<bool, DbError> {
        let key = Self::advisory_lock_key(address, network);
        let sql = format!("SELECT pg_try_advisory_lock({}) AS got", key);
        let row = self
            .conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(row
            .and_then(|r| r.try_get::<bool>("", "got").ok())
            .unwrap_or(false))
    }

    pub async fn release_advisory_lock(
        &self,
        address: &str,
        network: &str,
    ) -> Result<(), DbError> {
        let key = Self::advisory_lock_key(address, network);
        let sql = format!("SELECT pg_advisory_unlock({})", key);
        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Deterministic 64-bit lock key. Must match the API's derivation
    /// (`api::services::address_monitor_service`) so the indexer worker
    /// and the API on-demand seeder mutex against each other.
    fn advisory_lock_key(address: &str, network: &str) -> i64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        address.hash(&mut h);
        network.hash(&mut h);
        h.finish() as i64
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
