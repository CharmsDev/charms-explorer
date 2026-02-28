//! Mempool UTXO tracking for monitored addresses.
//! For every mempool transaction, checks if any inputs spend UTXOs from
//! monitored addresses (records mempool_spend) and if any outputs go to
//! monitored addresses (inserts address_utxos with block_height = 0).

use std::collections::HashSet;

use bitcoincore_rpc::bitcoin::{self, consensus::deserialize};

use crate::infrastructure::persistence::repositories::utxo_repository::UtxoInsert;
use crate::infrastructure::persistence::repositories::{
    MempoolSpendsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::utils::logging;

/// Track UTXO changes from a raw mempool transaction for monitored addresses.
/// - Inputs spending monitored UTXOs → record in mempool_spends
/// - Outputs to monitored addresses → insert in address_utxos with block_height=0 (unconfirmed)
pub async fn track_mempool_utxos(
    txid: &str,
    raw_hex: &str,
    network: &str,
    monitored_set: &HashSet<String>,
    utxo_repository: &UtxoRepository,
    mempool_spends_repository: &MempoolSpendsRepository,
) {
    if monitored_set.is_empty() {
        return;
    }

    let tx_bytes = match hex::decode(raw_hex) {
        Ok(b) => b,
        Err(_) => return,
    };

    let tx: bitcoin::Transaction = match deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(_) => return,
    };

    let btc_network = match network {
        "mainnet" => bitcoin::Network::Bitcoin,
        "testnet4" => bitcoin::Network::Testnet,
        _ => bitcoin::Network::Testnet,
    };

    // 1. Record mempool spends for inputs that consume monitored UTXOs
    let spends: Vec<(String, String, i32)> = tx
        .input
        .iter()
        .filter_map(|inp| {
            let prev_txid = inp.previous_output.txid.to_string();
            let prev_vout = inp.previous_output.vout as i32;
            if prev_txid == "0000000000000000000000000000000000000000000000000000000000000000" {
                None
            } else {
                Some((txid.to_string(), prev_txid, prev_vout))
            }
        })
        .collect();

    if !spends.is_empty() {
        if let Err(e) = mempool_spends_repository
            .record_spends_batch(&spends, network)
            .await
        {
            logging::log_debug(&format!(
                "[{}] Mempool UTXO tracker: failed to record spends for {}: {}",
                network, txid, e
            ));
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
