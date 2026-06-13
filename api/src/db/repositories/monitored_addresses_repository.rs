// Monitored addresses database operations implementation
// Uses SeaORM ORM for CRUD. Advisory lock functions use ConnectionTrait
// since pg_try_advisory_lock has no ORM equivalent.

use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Statement,
};

use crate::entity::monitored_addresses;

/// Repository for monitored_addresses table (API side).
/// Handles checking, registering, and seeding addresses for on-demand monitoring.
#[derive(Clone)]
pub struct MonitoredAddressesRepository {
    conn: DatabaseConnection,
}

impl MonitoredAddressesRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Check if an address is already monitored.
    pub async fn is_monitored(&self, address: &str, network: &str) -> Result<bool, String> {
        let result = monitored_addresses::Entity::find()
            .filter(monitored_addresses::Column::Address.eq(address))
            .filter(monitored_addresses::Column::Network.eq(network))
            .one(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        Ok(result.is_some())
    }

    /// Register an address for monitoring with seed data.
    /// Sets seeded_at and seed_height to indicate the address has been initialized.
    /// Uses on_conflict to upsert.
    pub async fn register_seeded(
        &self,
        address: &str,
        network: &str,
        seed_height: i32,
        seed_block_hash: Option<&str>,
    ) -> Result<bool, String> {
        let now = chrono::Utc::now();
        let model = monitored_addresses::ActiveModel {
            address: Set(address.to_string()),
            network: Set(network.to_string()),
            source: Set("api".to_string()),
            seeded_at: Set(Some(now)),
            seed_height: Set(Some(seed_height)),
            seed_block_hash: Set(seed_block_hash.map(|s| s.to_string())),
            created_at: Set(now),
        };

        let result = monitored_addresses::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([
                    monitored_addresses::Column::Address,
                    monitored_addresses::Column::Network,
                ])
                .update_columns([
                    monitored_addresses::Column::SeededAt,
                    monitored_addresses::Column::SeedHeight,
                    monitored_addresses::Column::SeedBlockHash,
                ])
                .to_owned(),
            )
            .exec(&self.conn)
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(sea_orm::DbErr::RecordNotInserted) => Ok(false),
            Err(e) => Err(format!("DB insert failed: {}", e)),
        }
    }

    /// Check if an address is monitored AND has been seeded with UTXOs AND
    /// the seed cursor is still consistent with the indexer's chain view.
    ///
    /// Returns false in any of these cases:
    /// - address not monitored
    /// - `seeded_at IS NULL` (indexer/backfill row, no UTXOs fetched yet)
    /// - `seed_block_hash` is set but does NOT match the indexer's stored
    ///   `block_status.block_hash` at `seed_height` — a reorg between the
    ///   seed and the first indexed block invalidated the snapshot.
    ///
    /// Returning false in the mismatch case forces the caller to re-seed,
    /// closing the Maestro↔node handoff gap.
    pub async fn is_seeded(&self, address: &str, network: &str) -> Result<bool, String> {
        let result = monitored_addresses::Entity::find()
            .filter(monitored_addresses::Column::Address.eq(address))
            .filter(monitored_addresses::Column::Network.eq(network))
            .one(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        let Some(model) = result else {
            return Ok(false);
        };
        if model.seeded_at.is_none() {
            return Ok(false);
        }

        // Legacy rows (no hash captured) are assumed valid.
        let (Some(expected_hash), Some(height)) = (&model.seed_block_hash, model.seed_height)
        else {
            return Ok(true);
        };

        self.seed_cursor_matches(network, height, expected_hash).await
    }

    /// Verify that the seed cursor still matches the indexer's view.
    /// Returns true when:
    /// - the indexer hasn't reached `height` yet (no `block_status` row), OR
    /// - the indexer's stored block_hash equals `expected_hash`.
    async fn seed_cursor_matches(
        &self,
        network: &str,
        height: i32,
        expected_hash: &str,
    ) -> Result<bool, String> {
        use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct Row {
            block_hash: Option<String>,
        }

        let row = Row::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT block_hash FROM block_status \
             WHERE block_height = $1 AND network = $2 LIMIT 1",
            [height.into(), network.into()],
        ))
        .one(&self.conn)
        .await
        .map_err(|e| format!("seed cursor lookup: {}", e))?;

        match row.and_then(|r| r.block_hash) {
            Some(stored) => Ok(stored == expected_hash),
            None => Ok(true),
        }
    }

    /// Acquire an advisory lock for seeding an address (prevents race conditions).
    /// Returns true if lock was acquired, false if another process holds it.
    /// Note: pg_try_advisory_lock has no ORM equivalent — uses ConnectionTrait.
    pub async fn try_advisory_lock(&self, address: &str, network: &str) -> Result<bool, String> {
        let lock_key = Self::advisory_lock_key(address, network);
        let sql = format!("SELECT pg_try_advisory_lock({})", lock_key);

        let result = self
            .conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| format!("Advisory lock failed: {}", e))?;

        match result {
            Some(row) => {
                let locked: bool = row
                    .try_get("", "pg_try_advisory_lock")
                    .map_err(|e| format!("Failed to read lock result: {}", e))?;
                Ok(locked)
            }
            None => Ok(false),
        }
    }

    /// Release an advisory lock after seeding.
    /// Note: pg_advisory_unlock has no ORM equivalent — uses ConnectionTrait.
    pub async fn release_advisory_lock(&self, address: &str, network: &str) -> Result<(), String> {
        let lock_key = Self::advisory_lock_key(address, network);
        let sql = format!("SELECT pg_advisory_unlock({})", lock_key);

        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| format!("Advisory unlock failed: {}", e))?;

        Ok(())
    }

    /// Generate a deterministic i64 lock key from address + network.
    fn advisory_lock_key(address: &str, network: &str) -> i64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        address.hash(&mut hasher);
        network.hash(&mut hasher);
        hasher.finish() as i64
    }
}
