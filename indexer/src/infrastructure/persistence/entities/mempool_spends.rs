//! SeaORM Entity for mempool_spends table
//! Tracks which UTXOs are being spent by unconfirmed mempool transactions.
//! Used to compute "available balance" = confirmed UTXOs not spent in mempool.

use chrono::DateTime;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "mempool_spends")]
pub struct Model {
    /// The txid of the mempool transaction that is spending the UTXO
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub spent_txid: String,
    /// The vout index of the UTXO being spent
    #[sea_orm(primary_key, auto_increment = false)]
    pub spent_vout: i32,
    /// Network (mainnet / testnet4)
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub network: String,
    /// The txid of the spending mempool transaction
    #[sea_orm(column_type = "Text")]
    pub spending_txid: String,
    /// When this spend was first detected in mempool
    pub detected_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
