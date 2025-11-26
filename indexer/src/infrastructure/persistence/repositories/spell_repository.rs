// [RJJ-S01] SpellRepository - Infrastructure layer for spell persistence

use crate::domain::models::Spell;
use crate::infrastructure::persistence::entities::spells;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, DbErr};

/// Repository for spell persistence operations
#[derive(Clone)]
pub struct SpellRepository {
    conn: DatabaseConnection,
}

impl SpellRepository {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Save a single spell to the database
    /// [RJJ-S01] Spells are saved before parsing charms
    /// Spells don't have addresses (they are OP_RETURN outputs)
    pub async fn save_spell(&self, spell: &Spell) -> Result<(), DbErr> {
        // Create active model from spell
        let spell_model = spells::ActiveModel {
            txid: Set(spell.txid.clone()),
            block_height: Set(spell.block_height as i32),
            data: Set(spell.data.clone()),
            date_created: Set(spell.date_created),
            asset_type: Set(spell.asset_type.clone()),
            blockchain: Set(spell.blockchain.clone()),
            network: Set(spell.network.clone()),
        };

        // Try to insert the spell, handle duplicate key violations gracefully
        match spell_model.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Spell already exists, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate it
                    Err(e)
                }
            }
        }
    }

    /// Save multiple spells in a batch operation
    /// [RJJ-S01] Optimized for bulk spell insertion
    pub async fn save_batch(
        &self,
        spells: Vec<(
            String, // txid
            u64,    // block_height
            serde_json::Value, // data
            String, // blockchain
            String, // network
        )>,
    ) -> Result<(), DbErr> {
        if spells.is_empty() {
            return Ok(());
        }

        // Create active models for all spells
        let now = chrono::Utc::now().naive_utc();
        let models: Vec<spells::ActiveModel> = spells
            .into_iter()
            .map(|(txid, block_height, data, blockchain, network)| {
                spells::ActiveModel {
                    txid: Set(txid),
                    block_height: Set(block_height as i32),
                    data: Set(data),
                    date_created: Set(now),
                    asset_type: Set("spell".to_string()),
                    blockchain: Set(blockchain),
                    network: Set(network),
                }
            })
            .collect();

        // Try to insert all spells, handle duplicate key violations gracefully
        match spells::Entity::insert_many(models).exec(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if the error is a duplicate key violation
                if e.to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    // Spells already exist, this is not an error
                    Ok(())
                } else {
                    // If it's not a duplicate key error, propagate it
                    Err(e)
                }
            }
        }
    }

    /// Find a spell by transaction ID
    pub async fn find_by_txid(&self, txid: &str) -> Result<Option<Spell>, DbErr> {
        let spell_entity = spells::Entity::find_by_id(txid).one(&self.conn).await?;

        Ok(spell_entity.map(|entity| self.to_domain_model(entity)))
    }

    /// Convert database entity to domain model
    fn to_domain_model(&self, entity: spells::Model) -> Spell {
        Spell::new(
            entity.txid,
            entity.block_height as u64,
            entity.data,
            entity.date_created,
            entity.asset_type,
            entity.blockchain,
            entity.network,
        )
    }
}
