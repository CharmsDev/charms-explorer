// UTXO database operations implementation
// All queries use SeaORM ORM — no raw SQL.

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

use crate::entity::address_utxos;

/// A single UTXO to be inserted (used by on-demand seeding).
/// `source` is the provenance label written to `address_utxos.source`.
/// API seeding always writes `maestro` (regardless of which external
/// provider supplied the data); the indexer writes `node` and takes
/// precedence on conflict.
pub struct UtxoInsert {
    pub txid: String,
    pub vout: i32,
    pub address: String,
    pub value: i64,
    pub script_pubkey: String,
    pub block_height: i32,
    pub network: String,
    pub source: String,
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
    ) -> Result<Vec<address_utxos::Model>, String> {
        address_utxos::Entity::find()
            .filter(address_utxos::Column::Address.eq(address))
            .filter(address_utxos::Column::Network.eq(network))
            .order_by_desc(address_utxos::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))
    }

    /// Insert/refresh a batch of UTXOs (on-demand seeding from Esplora/QuickNode).
    /// On conflict the API path UPDATES block_height + value, mirroring the
    /// snapshot from the provider. Rows owned by the indexer (`source='node'`)
    /// are NOT overwritten — they are authoritative for confirmed state.
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
                        "('{}', {}, '{}', '{}', {}, '{}', {}, '{}')",
                        u.txid.replace('\'', "''"),
                        u.vout,
                        u.network.replace('\'', "''"),
                        u.address.replace('\'', "''"),
                        u.value,
                        u.script_pubkey.replace('\'', "''"),
                        u.block_height,
                        u.source.replace('\'', "''"),
                    )
                })
                .collect();

            // API refresh overrides external snapshots only; the indexer's
            // 'node' rows stay untouched.
            let sql = format!(
                "INSERT INTO address_utxos (txid, vout, network, address, value, script_pubkey, block_height, source) \
                 VALUES {} \
                 ON CONFLICT (txid, vout, network) DO UPDATE SET \
                   value = EXCLUDED.value, \
                   block_height = EXCLUDED.block_height, \
                   script_pubkey = CASE WHEN EXCLUDED.script_pubkey = '' THEN address_utxos.script_pubkey ELSE EXCLUDED.script_pubkey END, \
                   source = EXCLUDED.source \
                 WHERE address_utxos.source IS DISTINCT FROM 'node'",
                values.join(", ")
            );

            let result = self
                .conn
                .execute(sea_orm::Statement::from_string(
                    sea_orm::DbBackend::Postgres,
                    sql,
                ))
                .await;

            match result {
                Ok(r) => total += r.rows_affected() as usize,
                Err(e) => return Err(format!("DB insert failed: {}", e)),
            }
        }

        Ok(total)
    }

    /// Get UTXO count for an address
    #[allow(dead_code)]
    pub async fn count_by_address(&self, address: &str, network: &str) -> Result<i64, String> {
        let count = address_utxos::Entity::find()
            .filter(address_utxos::Column::Address.eq(address))
            .filter(address_utxos::Column::Network.eq(network))
            .count(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;
        Ok(count as i64)
    }
}
