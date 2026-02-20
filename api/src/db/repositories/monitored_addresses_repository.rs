use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

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
        let sql = format!(
            "SELECT 1 FROM monitored_addresses WHERE address = '{}' AND network = '{}' LIMIT 1",
            address.replace('\'', "''"),
            network.replace('\'', "''"),
        );

        let result = self
            .conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        Ok(result.is_some())
    }

    /// Register an address for monitoring with seed data.
    /// Sets seeded_at and seed_height to indicate the address has been initialized.
    pub async fn register_seeded(
        &self,
        address: &str,
        network: &str,
        seed_height: i32,
    ) -> Result<bool, String> {
        let sql = format!(
            "INSERT INTO monitored_addresses (address, network, source, seeded_at, seed_height, created_at) \
             VALUES ('{}', '{}', 'api', NOW(), {}, NOW()) \
             ON CONFLICT (address, network) DO UPDATE SET seeded_at = NOW(), seed_height = {}",
            address.replace('\'', "''"),
            network.replace('\'', "''"),
            seed_height,
            seed_height,
        );

        let result = self
            .conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| format!("DB insert failed: {}", e))?;

        Ok(result.rows_affected() > 0)
    }

    /// Acquire an advisory lock for seeding an address (prevents race conditions).
    /// Returns true if lock was acquired, false if another process holds it.
    pub async fn try_advisory_lock(&self, address: &str, network: &str) -> Result<bool, String> {
        // Use a hash of address+network as the lock key
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
    pub async fn release_advisory_lock(
        &self,
        address: &str,
        network: &str,
    ) -> Result<(), String> {
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
