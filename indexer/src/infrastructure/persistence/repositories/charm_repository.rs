use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

use crate::domain::models::Charm;
use crate::infrastructure::persistence::entities::charms;
use crate::infrastructure::persistence::error::DbError;

/// Repository for charm operations
#[derive(Clone)]
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Create a new CharmRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Save a charm
    pub async fn save_charm(&self, charm: &Charm) -> Result<(), DbError> {
        // Create a new active model
        let charm_model = charms::ActiveModel {
            txid: Set(charm.txid.clone()),
            charmid: Set(charm.charmid.clone()),
            block_height: Set(charm.block_height as i32),
            data: Set(charm.data.clone()),
            date_created: Set(charm.date_created),
            asset_type: Set(charm.asset_type.clone()),
        };

        // Insert or update the charm
        charm_model.insert(&self.conn).await?;

        Ok(())
    }

    /// Get a charm by its transaction ID
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<Charm>, DbError> {
        // Query the database for the charm
        let result = charms::Entity::find_by_id(txid).one(&self.conn).await?;

        // Convert to domain model if found
        Ok(result.map(|c| self.to_domain_model(c)))
    }

    /// Find charms by charm ID
    pub async fn find_by_charmid(&self, charmid: &str) -> Result<Vec<Charm>, DbError> {
        // Query the database for charms with the given charm ID
        let results = charms::Entity::find()
            .filter(charms::Column::Charmid.eq(charmid))
            .all(&self.conn)
            .await?;

        // Convert to domain models
        Ok(results
            .into_iter()
            .map(|c| self.to_domain_model(c))
            .collect())
    }

    /// Find charms by asset type
    pub async fn find_by_asset_type(&self, asset_type: &str) -> Result<Vec<Charm>, DbError> {
        // Query the database for charms with the given asset type
        let results = charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .all(&self.conn)
            .await?;

        // Convert to domain models
        Ok(results
            .into_iter()
            .map(|c| self.to_domain_model(c))
            .collect())
    }

    /// Find charms with pagination
    pub async fn find_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Charm>, u64), DbError> {
        // Create a paginator
        let paginator = charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .paginate(&self.conn, page_size);

        // Get the total number of pages
        let num_pages = paginator.num_pages().await?;

        // Get the current page
        let results = paginator.fetch_page(page).await?;

        // Convert to domain models
        let charms = results
            .into_iter()
            .map(|c| self.to_domain_model(c))
            .collect();

        Ok((charms, num_pages))
    }

    /// Save multiple charms in a batch
    pub async fn save_batch(
        &self,
        charms: Vec<(String, String, u64, serde_json::Value, String)>,
    ) -> Result<(), DbError> {
        // Create active models for each charm
        let now = chrono::Utc::now().naive_utc();
        let models: Vec<charms::ActiveModel> = charms
            .into_iter()
            .map(
                |(txid, charmid, block_height, data, asset_type)| charms::ActiveModel {
                    txid: Set(txid),
                    charmid: Set(charmid),
                    block_height: Set(block_height as i32),
                    data: Set(data),
                    date_created: Set(now),
                    asset_type: Set(asset_type),
                },
            )
            .collect();

        // Insert all charms
        charms::Entity::insert_many(models).exec(&self.conn).await?;

        Ok(())
    }

    /// Convert a database entity to a domain model
    fn to_domain_model(&self, entity: charms::Model) -> Charm {
        Charm::new(
            entity.txid,
            entity.charmid,
            entity.block_height as u64,
            entity.data,
            entity.date_created,
            entity.asset_type,
        )
    }
}
