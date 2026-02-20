use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement};

/// Row returned from address_utxos queries
#[derive(Debug, FromQueryResult)]
pub struct UtxoRow {
    pub txid: String,
    pub vout: i32,
    pub address: String,
    pub value: i64,
    pub script_pubkey: String,
    pub block_height: i32,
}

/// A single UTXO to be inserted (used by on-demand seeding)
pub struct UtxoInsert {
    pub txid: String,
    pub vout: i32,
    pub address: String,
    pub value: i64,
    pub script_pubkey: String,
    pub block_height: i32,
    pub network: String,
}

/// Repository for the address_utxos table
#[derive(Clone)]
pub struct UtxoRepository {
    conn: DatabaseConnection,
}

impl UtxoRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get all UTXOs for a given address and network
    pub async fn get_by_address(
        &self,
        address: &str,
        network: &str,
    ) -> Result<Vec<UtxoRow>, String> {
        let sql = format!(
            "SELECT txid, vout, address, value, script_pubkey, block_height FROM address_utxos WHERE address = '{}' AND network = '{}'",
            address.replace('\'', "''"),
            network.replace('\'', "''")
        );

        let rows = UtxoRow::find_by_statement(Statement::from_string(DbBackend::Postgres, sql))
            .all(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        Ok(rows)
    }

    /// Insert a batch of UTXOs (used by on-demand seeding from QuickNode)
    pub async fn insert_batch(&self, utxos: &[UtxoInsert]) -> Result<usize, String> {
        if utxos.is_empty() {
            return Ok(0);
        }

        let mut total = 0usize;
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
                "INSERT INTO address_utxos (txid, vout, network, address, value, script_pubkey, block_height) \
                 VALUES {} ON CONFLICT (txid, vout, network) DO NOTHING",
                values.join(", ")
            );

            let result = self
                .conn
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await
                .map_err(|e| format!("DB insert failed: {}", e))?;

            total += result.rows_affected() as usize;
        }

        Ok(total)
    }

    /// Get UTXO count for an address
    pub async fn count_by_address(&self, address: &str, network: &str) -> Result<i64, String> {
        let sql = format!(
            "SELECT COUNT(*) as cnt FROM address_utxos WHERE address = '{}' AND network = '{}'",
            address.replace('\'', "''"),
            network.replace('\'', "''")
        );

        let result = self
            .conn
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        match result {
            Some(row) => {
                let count: i64 = row
                    .try_get("", "cnt")
                    .map_err(|e| format!("Failed to read count: {}", e))?;
                Ok(count)
            }
            None => Ok(0),
        }
    }
}
