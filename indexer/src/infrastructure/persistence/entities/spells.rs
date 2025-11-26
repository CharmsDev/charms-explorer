//! SeaORM Entity for Spells table
//! [RJJ-S01] New entity to store spell data (output 0 of charm transactions)
//! Spells are OP_RETURN outputs and don't have addresses

use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "spells")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Text")]
    pub txid: String,
    pub block_height: i32,
    pub data: Value,
    pub date_created: NaiveDateTime,
    #[sea_orm(column_type = "Text")]
    pub asset_type: String,
    #[sea_orm(column_type = "Text")]
    pub blockchain: String,
    #[sea_orm(column_type = "Text")]
    pub network: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
