use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, ConnectionTrait, Statement, DbBackend,
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

    /// Optimize the session for high-throughput writer by toggling synchronous_commit
    pub async fn set_synchronous_commit(&self, on: bool) -> Result<(), DbError> {
        let value = if on { "on" } else { "off" };
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!("SET synchronous_commit = {};", value),
        );
        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// Save a charm
    pub async fn save_charm(&self, charm: &Charm) -> Result<(), DbError> {
        // Check if charm already exists
        if let Some(_existing) = self.get_by_txid(&charm.txid).await? {
            // Charm already exists, skip insertion
            return Ok(());
        }

        // Create a new active model
        let charm_model = charms::ActiveModel {
            txid: Set(charm.txid.clone()),
            charmid: Set(charm.charmid.clone()),
            block_height: Set(charm.block_height as i32),
            data: Set(charm.data.clone()),
            date_created: Set(charm.date_created),
            asset_type: Set(charm.asset_type.clone()),
            blockchain: Set(charm.blockchain.clone()),
            network: Set(charm.network.clone()),
            address: Set(charm.address.clone()),
        };

        // Try to insert the charm, handle duplicate key violations gracefully
        match charm_model.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Charm already exists, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate the original error
                    Err(e.into())
                }
            }
        }
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

    /// Find charms by blockchain and network
    pub async fn find_by_blockchain_network(
        &self,
        blockchain: &str,
        network: &str,
    ) -> Result<Vec<Charm>, DbError> {
        // Query the database for charms with the given blockchain and network
        let results = charms::Entity::find()
            .filter(charms::Column::Blockchain.eq(blockchain))
            .filter(charms::Column::Network.eq(network))
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
        charms: Vec<(
            String,
            String,
            u64,
            serde_json::Value,
            String,
            String,
            String,
        )>,
    ) -> Result<(), DbError> {
        // Skip individual existence checks - let database handle duplicates
        if charms.is_empty() {
            return Ok(());
        }

        // Create active models for all charms
        let now = chrono::Utc::now().naive_utc();
        let models: Vec<charms::ActiveModel> = charms
            .into_iter()
            .map(
                |(txid, charmid, block_height, data, asset_type, blockchain, network)| {
                    charms::ActiveModel {
                        txid: Set(txid),
                        charmid: Set(charmid),
                        block_height: Set(block_height as i32),
                        data: Set(data),
                        date_created: Set(now),
                        asset_type: Set(asset_type),
                        blockchain: Set(blockchain),
                        network: Set(network),
                        address: Set(None), // For batch operations, address is not available
                    }
                },
            )
            .collect();

        // Try to insert all charms, handle duplicate key violations gracefully
        match charms::Entity::insert_many(models).exec(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Some charms already exist, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate the original error
                    Err(e.into())
                }
            }
        }
    }

    /// Get charms by Bitcoin address
    pub async fn get_charms_by_address(&self, address: &str, network: Option<&str>) -> Result<Vec<Charm>, DbError> {
        let mut query = charms::Entity::find()
            .filter(charms::Column::Address.eq(address));
        
        // Filter by network if provided
        if let Some(net) = network {
            query = query.filter(charms::Column::Network.eq(net));
        }
        
        let entities = query
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(entities.into_iter().map(|e| self.to_domain_model(e)).collect())
    }

    /// Get charm count by Bitcoin address
    pub async fn get_charm_count_by_address(&self, address: &str, network: Option<&str>) -> Result<u64, DbError> {
        let mut query = charms::Entity::find()
            .filter(charms::Column::Address.eq(address));
        
        // Filter by network if provided
        if let Some(net) = network {
            query = query.filter(charms::Column::Network.eq(net));
        }
        
        let count = query
            .count(&self.conn)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(count)
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
            entity.blockchain,
            entity.network,
            entity.address,
        )
    }
}
