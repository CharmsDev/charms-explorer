use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement,
};

use crate::domain::models::Charm;
use crate::infrastructure::persistence::entities::charms;
use crate::infrastructure::persistence::error::DbError;

/// Repository for charm operations
#[derive(Clone, Debug)]
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
        // [RJJ-S01] Removed charmid field, added app_id and amount
        let charm_model = charms::ActiveModel {
            txid: Set(charm.txid.clone()),
            vout: Set(charm.vout),
            block_height: Set(charm.block_height.map(|h| h as i32)),
            data: Set(charm.data.clone()),
            date_created: Set(charm.date_created),
            asset_type: Set(charm.asset_type.clone()),
            blockchain: Set(charm.blockchain.clone()),
            network: Set(charm.network.clone()),
            address: Set(charm.address.clone()),
            spent: Set(charm.spent),
            app_id: Set(charm.app_id.clone()),
            amount: Set(charm.amount),
            mempool_detected_at: Set(charm.mempool_detected_at),
            tags: Set(charm.tags.clone()),
            verified: Set(charm.verified),
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

    /// Get a charm by its transaction ID and vout
    /// [RJJ-S01] Updated: now requires both txid and vout (composite primary key)
    pub async fn get_by_txid_vout(&self, txid: &str, vout: i32) -> Result<Option<Charm>, DbError> {
        // Query the database for the charm using composite primary key
        let result = charms::Entity::find_by_id((txid.to_string(), vout))
            .one(&self.conn)
            .await?;

        // Convert to domain model if found
        Ok(result.map(|c| self.to_domain_model(c)))
    }

    /// Get a charm by its transaction ID (returns first match)
    /// [RJJ-S01] Note: Since primary key is now (txid, vout), this returns the first charm found
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<Charm>, DbError> {
        // Query the database for the first charm with this txid
        let result = charms::Entity::find()
            .filter(charms::Column::Txid.eq(txid))
            .one(&self.conn)
            .await?;

        // Convert to domain model if found
        Ok(result.map(|c| self.to_domain_model(c)))
    }

    /// Find charms by app_id
    /// [RJJ-S01] Updated: replaced charmid with app_id
    pub async fn find_by_app_id(&self, app_id: &str) -> Result<Vec<Charm>, DbError> {
        // Query the database for charms with the given app_id
        let results = charms::Entity::find()
            .filter(charms::Column::AppId.eq(app_id))
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
    /// [RJJ-S01] Updated signature: removed charmid, added vout, app_id, and amount
    /// [RJJ-ADDRESS] Added address field
    /// [RJJ-DEX] Added tags field
    pub async fn save_batch(
        &self,
        charms: Vec<(
            String,            // txid
            i32,               // vout
            u64,               // block_height
            serde_json::Value, // data
            String,            // asset_type
            String,            // blockchain
            String,            // network
            Option<String>,    // address
            String,            // app_id
            i64,               // amount
            Option<String>,    // tags
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
                |(
                    txid,
                    vout,
                    block_height,
                    data,
                    asset_type,
                    blockchain,
                    network,
                    address,
                    app_id,
                    amount,
                    tags,
                )| {
                    // [RJJ-S01] Removed charmid, vout now comes from input, added app_id and amount
                    // [RJJ-DEX] Added tags field
                    charms::ActiveModel {
                        txid: Set(txid),
                        vout: Set(vout),
                        block_height: Set(Some(block_height as i32)),
                        data: Set(data),
                        date_created: Set(now),
                        asset_type: Set(asset_type),
                        blockchain: Set(blockchain),
                        network: Set(network),
                        address: Set(address), // [RJJ-ADDRESS] Now includes address from extraction
                        spent: Set(false),     // New charms are unspent by default
                        app_id: Set(app_id),
                        amount: Set(amount),
                        mempool_detected_at: Set(None),
                        tags: Set(tags),     // [RJJ-DEX] Tags from detection
                        verified: Set(true), // Charms are verified during extraction
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
    pub async fn get_charms_by_address(
        &self,
        address: &str,
        network: Option<&str>,
    ) -> Result<Vec<Charm>, DbError> {
        let mut query = charms::Entity::find().filter(charms::Column::Address.eq(address));

        // Filter by network if provided
        if let Some(net) = network {
            query = query.filter(charms::Column::Network.eq(net));
        }

        let entities = query
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(entities
            .into_iter()
            .map(|e| self.to_domain_model(e))
            .collect())
    }

    /// Get charm count by Bitcoin address
    pub async fn get_charm_count_by_address(
        &self,
        address: &str,
        network: Option<&str>,
    ) -> Result<u64, DbError> {
        let mut query = charms::Entity::find().filter(charms::Column::Address.eq(address));

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
    /// [RJJ-S01] Removed charmid from conversion, added app_id and amount
    fn to_domain_model(&self, entity: charms::Model) -> Charm {
        let mut charm = Charm::new(
            entity.txid,
            entity.vout,
            entity.block_height.map(|h| h as u64),
            entity.data,
            entity.date_created,
            entity.asset_type,
            entity.blockchain,
            entity.network,
            entity.address,
            entity.spent,
            entity.app_id,
            entity.amount,
        );
        charm.mempool_detected_at = entity.mempool_detected_at;
        charm.tags = entity.tags;
        charm
    }

    /// Find charms by tag (searches for tag in comma-separated tags field)
    pub async fn find_by_tag(&self, tag: &str) -> Result<Vec<Charm>, DbError> {
        // Use LIKE to find charms containing the tag
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT * FROM charms WHERE tags LIKE '%{}%' ORDER BY block_height DESC",
                tag.replace('\'', "''") // Escape single quotes
            ),
        );

        let results = charms::Entity::find()
            .from_raw_sql(stmt)
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|c| self.to_domain_model(c))
            .collect())
    }

    /// Mark a charm as spent by its txid and vout
    /// [RJJ-S01] Updated: now requires both txid and vout (composite primary key)
    pub async fn mark_charm_as_spent(&self, txid: &str, vout: i32) -> Result<(), DbError> {
        use sea_orm::ActiveValue::Set;

        // Find the charm using composite primary key
        if let Some(charm) = charms::Entity::find_by_id((txid.to_string(), vout))
            .one(&self.conn)
            .await?
        {
            // Update only if not already spent
            if !charm.spent {
                let mut active_model: charms::ActiveModel = charm.into();
                active_model.spent = Set(true);
                active_model.update(&self.conn).await?;
            }
        }
        Ok(())
    }

    /// Mark multiple charms as spent in a batch
    pub async fn mark_charms_as_spent_batch(&self, txids: Vec<String>) -> Result<(), DbError> {
        if txids.is_empty() {
            return Ok(());
        }

        // Use raw SQL for efficient batch update
        let txids_str = txids
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(",");

        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!(
                "UPDATE charms SET spent = true WHERE txid IN ({}) AND spent = false",
                txids_str
            ),
        );

        self.conn
            .execute(stmt)
            .await
            .map(|_| ())
            .map_err(|e| DbError::QueryError(e.to_string()))
    }

    /// [RJJ-STATS-HOLDERS] Get charm info for stats_holders updates before marking as spent
    /// Returns (app_id, address, amount) for charms that will be marked as spent
    pub async fn get_charms_for_spent_update(
        &self,
        txids: Vec<String>,
    ) -> Result<Vec<(String, String, i64)>, DbError> {
        if txids.is_empty() {
            return Ok(vec![]);
        }

        let results = charms::Entity::find()
            .filter(charms::Column::Txid.is_in(txids))
            .filter(charms::Column::Spent.eq(false))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .filter_map(|c| c.address.map(|addr| (c.app_id, addr, c.amount)))
            .collect())
    }

    /// Get amounts by txids for calculating net supply change
    /// Returns: Vec<(txid, app_id, amount)>
    pub async fn get_amounts_by_txids(
        &self,
        txids: &[String],
    ) -> Result<Vec<(String, String, u64)>, DbError> {
        if txids.is_empty() {
            return Ok(vec![]);
        }

        let results = charms::Entity::find()
            .filter(charms::Column::Txid.is_in(txids.iter().map(|s| s.as_str())))
            .all(&self.conn)
            .await?;

        Ok(results
            .into_iter()
            .map(|c| (c.txid, c.app_id, c.amount as u64))
            .collect())
    }
}
