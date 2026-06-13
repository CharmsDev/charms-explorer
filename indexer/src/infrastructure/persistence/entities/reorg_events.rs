//! SeaORM Entity for `reorg_events`. Audit trail of detected reorgs.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reorg_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Text")]
    pub network: String,
    pub from_height: i32,
    pub depth: i32,
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub detected_at: DateTimeWithTimeZone,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub recovered_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
