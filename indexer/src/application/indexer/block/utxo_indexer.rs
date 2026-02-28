//! UTXO index management for monitored addresses.
//! Registers charm holder addresses and tracks their BTC UTXOs.

use bitcoincore_rpc::bitcoin;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::infrastructure::persistence::repositories::utxo_repository::UtxoInsert;
use crate::infrastructure::persistence::repositories::{
    MonitoredAddressesRepository, UtxoRepository,
};
use crate::utils::logging;

use super::batch::CharmBatchItem;

/// Auto-register addresses from detected charms for monitoring.
/// Any address that holds a charm becomes a monitored address so that
/// the indexer tracks its BTC UTXOs in real time.
pub async fn register_charm_addresses(
    charm_batch: &[CharmBatchItem],
    network_id: &NetworkId,
    monitored_addresses_repository: &MonitoredAddressesRepository,
) {
    let addresses: Vec<String> = charm_batch
        .iter()
        .filter_map(|(_, _, _, _, _, _, _, address, _, _, _)| address.clone())
        .filter(|addr| !addr.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if addresses.is_empty() {
        return;
    }

    match monitored_addresses_repository
        .register_batch(&addresses, &network_id.name, "indexer")
        .await
    {
        Ok(new_count) => {
            if new_count > 0 {
                logging::log_info(&format!(
                    "[{}] 📡 Registered {} new monitored addresses from charms",
                    network_id.name, new_count
                ));
            }
        }
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] Failed to register charm addresses: {}",
                network_id.name, e
            ));
        }
    }
}

/// Update UTXO index for monitored addresses only.
/// 1. Load monitored address set
/// 2. Delete spent UTXOs
/// 3. Insert new UTXOs only for monitored addresses
pub async fn update_monitored_utxos(
    block: &bitcoin::Block,
    height: u64,
    network_id: &NetworkId,
    monitored_addresses_repository: &MonitoredAddressesRepository,
    utxo_repository: &UtxoRepository,
) -> Result<(), BlockProcessorError> {
    let network_str = &network_id.name;

    let monitored = match monitored_addresses_repository.load_set(network_str).await {
        Ok(set) => set,
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] Failed to load monitored addresses: {}, skipping UTXO index",
                network_str, e
            ));
            return Ok(());
        }
    };

    if monitored.is_empty() {
        return Ok(());
    }

    let btc_network = match network_str.as_str() {
        "mainnet" => bitcoin::Network::Bitcoin,
        "testnet4" => bitcoin::Network::Testnet,
        _ => bitcoin::Network::Testnet,
    };

    // 1. Collect spent UTXOs from inputs
    let mut spent: Vec<(String, i32)> = Vec::new();
    for tx in &block.txdata {
        if tx.is_coin_base() {
            continue;
        }
        for input in &tx.input {
            if !input.previous_output.is_null() {
                spent.push((
                    input.previous_output.txid.to_string(),
                    input.previous_output.vout as i32,
                ));
            }
        }
    }

    // 2. Collect new UTXOs — only for monitored addresses
    let mut new_utxos: Vec<UtxoInsert> = Vec::new();
    for tx in &block.txdata {
        let txid = tx.txid().to_string();
        for (vout, output) in tx.output.iter().enumerate() {
            if output.script_pubkey.is_provably_unspendable() {
                continue;
            }
            if let Ok(address) =
                bitcoin::Address::from_script(&output.script_pubkey, btc_network)
            {
                let addr_str = address.to_string();
                if monitored.contains(&addr_str) {
                    new_utxos.push(UtxoInsert {
                        txid: txid.clone(),
                        vout: vout as i32,
                        address: addr_str,
                        value: output.value as i64,
                        script_pubkey: format!("{:x}", output.script_pubkey),
                        block_height: height as i32,
                        network: network_str.clone(),
                    });
                }
            }
        }
    }

    // 3. Delete spent UTXOs
    if !spent.is_empty() {
        if let Err(e) = utxo_repository.delete_spent_batch(&spent, network_str).await {
            logging::log_warning(&format!(
                "[{}] Failed to delete spent UTXOs at block {}: {}",
                network_str, height, e
            ));
        }
    }

    // 4. Insert new UTXOs (only monitored)
    if !new_utxos.is_empty() {
        if let Err(e) = utxo_repository.insert_batch(&new_utxos).await {
            logging::log_warning(&format!(
                "[{}] Failed to insert UTXOs at block {}: {}",
                network_str, height, e
            ));
        }
    }

    Ok(())
}
