//! Repository for address_transactions table.
//! Stores BTC transaction history for monitored addresses.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::fmt;

use crate::infrastructure::persistence::error::DbError;

/// A single address transaction to insert
#[derive(Debug, Clone)]
pub struct AddressTxInsert {
    pub txid: String,
    pub address: String,
    pub network: String,
    pub direction: String, // "in" or "out"
    pub amount: i64,       // sats (positive)
    pub fee: i64,
    pub block_height: Option<i32>, // None = mempool
    pub block_time: Option<i64>,   // unix timestamp
    pub confirmations: i32,
}

#[derive(Clone)]
pub struct AddressTransactionsRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for AddressTransactionsRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddressTransactionsRepository")
            .finish_non_exhaustive()
    }
}

impl AddressTransactionsRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Insert a batch of address transactions (ON CONFLICT DO NOTHING)
    pub async fn insert_batch(&self, txs: &[AddressTxInsert]) -> Result<usize, DbError> {
        if txs.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0usize;
        for chunk in txs.chunks(500) {
            let values: Vec<String> = chunk
                .iter()
                .map(|t| {
                    let bh = match t.block_height {
                        Some(h) => h.to_string(),
                        None => "NULL".to_string(),
                    };
                    let bt = match t.block_time {
                        Some(ts) => ts.to_string(),
                        None => "NULL".to_string(),
                    };
                    format!(
                        "('{}', '{}', '{}', '{}', {}, {}, {}, {}, {})",
                        t.txid.replace('\'', "''"),
                        t.address.replace('\'', "''"),
                        t.network.replace('\'', "''"),
                        t.direction.replace('\'', "''"),
                        t.amount,
                        t.fee,
                        bh,
                        bt,
                        t.confirmations,
                    )
                })
                .collect();

            let sql = format!(
                "INSERT INTO address_transactions (txid, address, network, direction, amount, fee, block_height, block_time, confirmations) \
                 VALUES {} ON CONFLICT (txid, address, network) DO NOTHING",
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

    /// Update block_height and confirmations when a mempool tx gets confirmed
    pub async fn confirm_tx(
        &self,
        txid: &str,
        block_height: i32,
        block_time: i64,
        network: &str,
    ) -> Result<(), DbError> {
        let sql = format!(
            "UPDATE address_transactions SET block_height = {}, block_time = {}, confirmations = 1 \
             WHERE txid = '{}' AND network = '{}' AND block_height IS NULL",
            block_height,
            block_time,
            txid.replace('\'', "''"),
            network.replace('\'', "''"),
        );

        self.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }
}
