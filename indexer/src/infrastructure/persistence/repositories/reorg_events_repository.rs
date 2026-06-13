//! Repository for the `reorg_events` audit table.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::fmt;

use crate::config::NetworkId;
use crate::infrastructure::persistence::entities::reorg_events;
use crate::infrastructure::persistence::error::DbError;

#[derive(Clone)]
pub struct ReorgEventsRepository {
    conn: DatabaseConnection,
}

impl fmt::Debug for ReorgEventsRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReorgEventsRepository").finish_non_exhaustive()
    }
}

impl ReorgEventsRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Record a detected reorg. Returns the inserted row id.
    pub async fn record(
        &self,
        network_id: &NetworkId,
        from_height: i32,
        depth: i32,
    ) -> Result<i32, DbError> {
        let now = Utc::now();
        let active = reorg_events::ActiveModel {
            network: Set(network_id.name.clone()),
            from_height: Set(from_height),
            depth: Set(depth),
            detected_at: Set(now.into()),
            recovered_at: Set(None),
            ..Default::default()
        };
        let inserted = active.insert(&self.conn).await?;
        Ok(inserted.id)
    }

    /// Mark a previously-recorded reorg as recovered.
    pub async fn mark_recovered(&self, id: i32) -> Result<(), DbError> {
        let now = Utc::now();
        let existing = reorg_events::Entity::find()
            .filter(reorg_events::Column::Id.eq(id))
            .one(&self.conn)
            .await?;
        if let Some(model) = existing {
            let mut update: reorg_events::ActiveModel = model.into();
            update.recovered_at = Set(Some(now.into()));
            update.update(&self.conn).await?;
        }
        Ok(())
    }
}
