//! Extract the UTXOs consumed by a mempool tx so they can be recorded in
//! `mempool_spends`. Pure parsing, no I/O.

use bitcoincore_rpc::bitcoin::{self, consensus::deserialize};

const NULL_TXID: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Return `(spending_txid, spent_txid, spent_vout)` for every non-coinbase
/// input of an already-decoded tx. Preferred over [`extract_spends`] when the
/// caller has the decoded transaction to hand — the poll loop decodes each
/// mempool tx once and shares it across all consumers.
pub fn extract_spends_from_tx(
    tx: &bitcoin::Transaction,
    spending_txid: &str,
) -> Vec<(String, String, i32)> {
    tx.input
        .iter()
        .filter_map(|inp| {
            let prev_txid = inp.previous_output.txid.to_string();
            if prev_txid == NULL_TXID {
                return None;
            }
            Some((
                spending_txid.to_string(),
                prev_txid,
                inp.previous_output.vout as i32,
            ))
        })
        .collect()
}

/// Same as [`extract_spends_from_tx`] but decodes `raw_hex` first. Invalid hex
/// or an undecodable tx yields an empty vec.
#[allow(dead_code)]
pub fn extract_spends(raw_hex: &str, spending_txid: &str) -> Vec<(String, String, i32)> {
    let tx_bytes = match hex::decode(raw_hex) {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    let tx: bitcoin::Transaction = match deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    extract_spends_from_tx(&tx, spending_txid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two-input P2WPKH spend. Inputs are
    /// `1111…11:0` and `2222…22:3`.
    const TWO_INPUT_TX: &str = "0200000002\
        1111111111111111111111111111111111111111111111111111111111111111000000000000000000\
        2222222222222222222222222222222222222222222222222222222222222222030000000000000000\
        0100e1f50500000000160014000000000000000000000000000000000000000000000000";

    /// Coinbase: single input whose prevout is the null txid.
    const COINBASE_TX: &str = "0100000001\
        0000000000000000000000000000000000000000000000000000000000000000ffffffff0100ffffffff\
        0100f2052a0100000016001400000000000000000000000000000000000000000000000000";

    #[test]
    fn extracts_every_non_coinbase_input() {
        let spends = extract_spends(TWO_INPUT_TX, "spender");
        assert_eq!(
            spends,
            vec![
                ("spender".to_string(), "1111111111111111111111111111111111111111111111111111111111111111".to_string(), 0),
                ("spender".to_string(), "2222222222222222222222222222222222222222222222222222222222222222".to_string(), 3),
            ]
        );
    }

    #[test]
    fn skips_coinbase_null_prevout() {
        assert!(extract_spends(COINBASE_TX, "spender").is_empty());
    }

    /// The decoded-tx path and the raw-hex wrapper must agree — the poll loop
    /// switched to the former to avoid re-parsing, and the two must not drift.
    #[test]
    fn decoded_and_hex_paths_agree() {
        let bytes = hex::decode(TWO_INPUT_TX).expect("valid hex");
        let tx: bitcoin::Transaction = deserialize(&bytes).expect("valid tx");
        assert_eq!(
            extract_spends_from_tx(&tx, "spender"),
            extract_spends(TWO_INPUT_TX, "spender")
        );
    }

    #[test]
    fn undecodable_input_yields_no_spends() {
        assert!(extract_spends("not hex at all", "spender").is_empty());
        assert!(extract_spends("deadbeef", "spender").is_empty());
    }
}
