// Address transactions repository — query and insert transaction history for monitored addresses

use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::entity::address_transactions;

/// A single address transaction to insert (used by seeding from QuickNode)
pub struct AddressTxInsert {
    pub txid: String,
    pub address: String,
    pub network: String,
    pub direction: String,
    pub amount: i64,
    pub fee: i64,
    pub block_height: Option<i32>,
    pub block_time: Option<i64>,
    pub confirmations: i32,
}

#[derive(Clone)]
pub struct AddressTransactionsRepository {
    conn: DatabaseConnection,
}

impl AddressTransactionsRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Get all transactions for a given address and network, ordered by block_time desc
    pub async fn get_by_address(
        &self,
        address: &str,
        network: &str,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<address_transactions::Model>, u64), String> {
        let offset = (page.saturating_sub(1)) * page_size;

        // Get total count
        use sea_orm::PaginatorTrait;
        let total = address_transactions::Entity::find()
            .filter(address_transactions::Column::Address.eq(address))
            .filter(address_transactions::Column::Network.eq(network))
            .count(&self.conn)
            .await
            .map_err(|e| format!("DB count failed: {}", e))?;

        // Get paginated results
        let results = address_transactions::Entity::find()
            .filter(address_transactions::Column::Address.eq(address))
            .filter(address_transactions::Column::Network.eq(network))
            .order_by_desc(address_transactions::Column::BlockTime)
            .order_by_desc(address_transactions::Column::Txid)
            .offset(Some(offset))
            .limit(Some(page_size))
            .all(&self.conn)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

        Ok((results, total))
    }

    /// Insert a batch of address transactions (used by seeding from QuickNode bb_getAddress)
    pub async fn insert_batch(&self, txs: &[AddressTxInsert]) -> Result<usize, String> {
        if txs.is_empty() {
            return Ok(0);
        }

        let mut total = 0usize;
        for chunk in txs.chunks(500) {
            let models: Vec<address_transactions::ActiveModel> = chunk
                .iter()
                .map(|t| address_transactions::ActiveModel {
                    txid: Set(t.txid.clone()),
                    address: Set(t.address.clone()),
                    network: Set(t.network.clone()),
                    direction: Set(t.direction.clone()),
                    amount: Set(t.amount),
                    fee: Set(t.fee),
                    block_height: Set(t.block_height),
                    block_time: Set(t.block_time),
                    confirmations: Set(t.confirmations),
                    created_at: Set(chrono::Utc::now().into()),
                })
                .collect();

            let result = address_transactions::Entity::insert_many(models)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::columns([
                        address_transactions::Column::Txid,
                        address_transactions::Column::Address,
                        address_transactions::Column::Network,
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
}
