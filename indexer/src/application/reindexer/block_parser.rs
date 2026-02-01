//! Block parsing logic for reindexer

use bitcoin::consensus::deserialize;
use futures::stream::{self, StreamExt};

use super::types::{AssetBatch, CharmBatch, SpellBatch};
use crate::domain::services::charm::spell_detection::SpellDetector;

/// Concurrency for parallel transaction parsing
const PARSER_CONCURRENCY: usize = 64;

/// Parse transactions and extract spells, charms, and assets
pub async fn parse_transactions(
    transactions: Vec<(String, String, i64)>,
    block_height: u64,
    network: &str,
) -> (SpellBatch, CharmBatch, AssetBatch) {
    let network_clone = network.to_string();

    let results: Vec<_> = stream::iter(transactions)
        .map(|(txid, hex, _ordinal)| {
            let net = network_clone.clone();
            async move { SpellDetector::parse_spell_only(&txid, block_height, &hex, &net) }
        })
        .buffer_unordered(PARSER_CONCURRENCY)
        .collect()
        .await;

    let mut spells_batch = SpellBatch::new();
    let mut charms_batch = CharmBatch::new();
    let mut assets_batch = AssetBatch::new();

    for result in results {
        if let Ok((Some(spell), charms)) = result {
            spells_batch.push((
                spell.txid.clone(),
                spell.block_height,
                spell.data.clone(),
                spell.blockchain.clone(),
                spell.network.clone(),
            ));

            for charm in &charms {
                charms_batch.push((
                    charm.txid.clone(),
                    charm.vout,
                    block_height,
                    charm.data.clone(),
                    charm.asset_type.clone(),
                    "Bitcoin".to_string(),
                    network.to_string(),
                    charm.address.clone(),
                    charm.app_id.clone(),
                    charm.amount,
                    None,
                ));

                let charm_id = format!("{}:{}", charm.txid, charm.vout);
                assets_batch.push((
                    charm.app_id.clone(),
                    charm.txid.clone(),
                    charm.vout,
                    charm_id,
                    block_height,
                    charm.data.clone(),
                    charm.asset_type.clone(),
                    "Bitcoin".to_string(),
                    network.to_string(),
                ));
            }
        }
    }

    (spells_batch, charms_batch, assets_batch)
}

/// Extract spent txids from transaction hex data
pub fn extract_spent_txids(tx_hexes: &[String]) -> Vec<String> {
    let mut spent_txids = Vec::new();

    for hex in tx_hexes {
        if let Ok(tx_bytes) = hex::decode(hex) {
            if let Ok(tx) = deserialize::<bitcoin::Transaction>(&tx_bytes) {
                if !tx.is_coinbase() {
                    for input in &tx.input {
                        spent_txids.push(input.previous_output.txid.to_string());
                    }
                }
            }
        }
    }

    spent_txids
}
