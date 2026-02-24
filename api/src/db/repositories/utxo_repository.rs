// UTXO database operations implementation
// All queries use SeaORM ORM — no raw SQL.

use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

use crate::entity::address_utxos;

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
    ) -> Result<Vec<address_utxos::Model>, String> {
        address_utxos::Entity::find()
            .filter(address_utxos::Column::Address.eq(address))
            .filter(address_utxos::Column::Network.eq(network))
            .order_by_desc(address_utxos::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))
    }

    /// Insert a batch of UTXOs (used by on-demand seeding from QuickNode)
    /// Uses ON CONFLICT DO NOTHING via SeaORM's on_conflict
    pub async fn insert_batch(&self, utxos: &[UtxoInsert]) -> Result<usize, String> {
        if utxos.is_empty() {
            return Ok(0);
        }

        let mut total = 0usize;
        for chunk in utxos.chunks(500) {
            let models: Vec<address_utxos::ActiveModel> = chunk
                .iter()
                .map(|u| address_utxos::ActiveModel {
                    txid: Set(u.txid.clone()),
                    vout: Set(u.vout),
                    network: Set(u.network.clone()),
                    address: Set(u.address.clone()),
                    value: Set(u.value),
                    script_pubkey: Set(u.script_pubkey.clone()),
                    block_height: Set(u.block_height),
                })
                .collect();

            let result = address_utxos::Entity::insert_many(models)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::columns([
                        address_utxos::Column::Txid,
                        address_utxos::Column::Vout,
                        address_utxos::Column::Network,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec(&self.conn)
                .await;

            match result {
                Ok(_) => total += chunk.len(),
                Err(sea_orm::DbErr::RecordNotInserted) => {
                    // All rows conflicted — nothing inserted, not an error
                }
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
