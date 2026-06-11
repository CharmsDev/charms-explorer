//! Extract the UTXOs consumed by a mempool tx so they can be recorded in
//! `mempool_spends`. Pure parsing, no I/O.

use bitcoincore_rpc::bitcoin::{self, consensus::deserialize};

/// Return `(spending_txid, spent_txid, spent_vout)` for every non-coinbase
/// input in the tx encoded by `raw_hex`. Invalid hex / undecodable tx yields
/// an empty vec.
pub fn extract_spends(raw_hex: &str, spending_txid: &str) -> Vec<(String, String, i32)> {
    let tx_bytes = match hex::decode(raw_hex) {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    let tx: bitcoin::Transaction = match deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    tx.input
        .iter()
        .filter_map(|inp| {
            let prev_txid = inp.previous_output.txid.to_string();
            let prev_vout = inp.previous_output.vout as i32;
            if prev_txid == "0000000000000000000000000000000000000000000000000000000000000000" {
                None
            } else {
                Some((spending_txid.to_string(), prev_txid, prev_vout))
            }
        })
        .collect()
}
