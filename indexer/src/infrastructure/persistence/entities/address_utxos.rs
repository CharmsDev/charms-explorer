use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "address_utxos")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub txid: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub vout: i32,
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub network: String,
    #[sea_orm(column_type = "Text")]
    pub address: String,
    pub value: i64,
    #[sea_orm(column_type = "Text")]
    pub script_pubkey: String,
    pub block_height: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
