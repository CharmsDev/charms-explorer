//! Mempool UTXO tracking for monitored addresses.
//! For every mempool transaction, checks if any inputs spend UTXOs from
//! monitored addresses (records mempool_spend) and if any outputs go to
//! monitored addresses (inserts address_utxos with block_height = 0).

use std::collections::HashSet;

use bitcoincore_rpc::bitcoin;

use super::spend_extraction::extract_spends_from_tx;
use crate::infrastructure::persistence::repositories::utxo_repository::UtxoInsert;
use crate::infrastructure::persistence::repositories::{
    MempoolSpendsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::utils::logging;

/// Track UTXO changes from a decoded mempool transaction for monitored addresses.
/// - Inputs spent by this tx → record in mempool_spends
/// - Outputs to monitored addresses → insert in address_utxos with block_height=0 (unconfirmed)
///
/// Takes the already-decoded tx: the poll loop deserializes each mempool tx
/// once and shares it, instead of every consumer re-parsing the same hex.
///
/// Returns true when spends were written, so the charm processor can skip
/// re-inserting the identical rows for this same tx.
pub async fn track_mempool_utxos(
    txid: &str,
    tx: &bitcoin::Transaction,
    network: &str,
    monitored_set: &HashSet<String>,
    utxo_repository: &UtxoRepository,
    mempool_spends_repository: &MempoolSpendsRepository,
) -> bool {
    if monitored_set.is_empty() {
        return false;
    }

    let btc_network = match network {
        "mainnet" => bitcoin::Network::Bitcoin,
        "testnet4" => bitcoin::Network::Testnet,
        _ => bitcoin::Network::Testnet,
    };

    // 1. Record the UTXOs this tx consumes
    let spends = extract_spends_from_tx(tx, txid);
    let mut spends_recorded = false;

    if !spends.is_empty() {
        match mempool_spends_repository
            .record_spends_batch(&spends, network)
            .await
        {
            Ok(()) => spends_recorded = true,
            Err(e) => logging::log_debug(&format!(
                "[{}] Mempool UTXO tracker: failed to record spends for {}: {}",
                network, txid, e
            )),
        }
    }

    // 2. Insert new UTXOs for outputs going to monitored addresses
    let mut new_utxos: Vec<UtxoInsert> = Vec::new();
    for (vout, output) in tx.output.iter().enumerate() {
        if output.script_pubkey.is_provably_unspendable() {
            continue;
        }
        if let Ok(address) = bitcoin::Address::from_script(&output.script_pubkey, btc_network) {
            let addr_str = address.to_string();
            if monitored_set.contains(&addr_str) {
                new_utxos.push(UtxoInsert {
                    txid: txid.to_string(),
                    vout: vout as i32,
                    address: addr_str,
                    value: output.value as i64,
                    script_pubkey: format!("{:x}", output.script_pubkey),
                    block_height: 0, // 0 = unconfirmed/mempool
                    network: network.to_string(),
                    source: "node".to_string(),
                });
            }
        }
    }

    if !new_utxos.is_empty() {
        if let Err(e) = utxo_repository.insert_batch(&new_utxos).await {
            logging::log_debug(&format!(
                "[{}] Mempool UTXO tracker: failed to insert UTXOs for {}: {}",
                network, txid, e
            ));
        } else {
            logging::log_info(&format!(
                "[{}] 💰 Mempool: {} new UTXOs for monitored addresses from tx {}",
                network,
                new_utxos.len(),
                txid
            ));
        }
    }

    spends_recorded
}

/// Load the monitored address set (call periodically, not per-tx)
pub async fn load_monitored_set(
    network: &str,
    monitored_addresses_repository: &MonitoredAddressesRepository,
) -> HashSet<String> {
    match monitored_addresses_repository.load_seeded_set(network).await {
        Ok(set) => set,
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] Failed to load monitored set for mempool UTXO tracking: {}",
                network, e
            ));
            HashSet::new()
        }
    }
}
