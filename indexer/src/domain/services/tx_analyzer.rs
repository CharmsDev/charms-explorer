//! Unified transaction analyzer — single source of truth for charm detection.
//!
//! All code paths (block processing, mempool, reindex) use this module
//! to parse a raw transaction hex and extract charm/spell/DEX data.
//! No persistence happens here — callers decide how to save.

use serde_json::{json, Value};

use super::address_extractor::AddressExtractor;
use super::dex;
use super::native_charm_parser::{AssetInfo, NativeCharmParser};

/// Result of analyzing a single transaction.
/// Contains everything needed for persistence — callers just save it.
#[derive(Debug, Clone)]
pub struct AnalyzedTx {
    pub txid: String,
    pub charm_json: Value,
    pub app_id: String,
    pub asset_type: String,
    pub amount: i64,
    pub address: Option<String>,
    pub tags: Option<String>,
    pub dex_result: Option<dex::DexDetectionResult>,
    pub asset_infos: Vec<AssetInfo>,
    pub is_beaming: bool,
    pub version: u32,
}

/// Pure analysis: parse raw tx hex → Option<AnalyzedTx>.
/// Returns None if the tx does not contain a charm spell.
/// This is intentionally a free function, not a method on a struct,
/// because it needs no state — only the raw bytes and the network name.
pub fn analyze_tx(txid: &str, raw_hex: &str, network: &str) -> Option<AnalyzedTx> {
    // 1. Parse spell from OP_RETURN or Taproot witness
    let spell = match NativeCharmParser::extract_spell_no_verify(raw_hex) {
        Ok(s) => s,
        Err(_) => return None,
    };

    // 2. Build charm JSON (same structure used by all paths)
    let spell_json = serde_json::to_value(&spell).unwrap_or_default();
    let charm_json = json!({
        "type": "spell",
        "detected": true,
        "has_native_data": true,
        "native_data": spell_json,
        "version": "native_parser"
    });

    // 3. Extract asset info from spell outputs
    let asset_infos = NativeCharmParser::extract_asset_info(&spell);

    // 4. Derive primary app_id / asset_type / amount from first asset
    let (app_id, asset_type, amount) = if let Some(first) = asset_infos.first() {
        let atype = if first.app_id.starts_with("t/") {
            "token"
        } else if first.app_id.starts_with("n/") {
            "nft"
        } else if first.app_id.starts_with("B/") {
            "dapp"
        } else {
            "other"
        };
        (first.app_id.clone(), atype.to_string(), first.amount as i64)
    } else {
        ("other".to_string(), "spell".to_string(), 0i64)
    };

    // 5. Extract holder address
    let address = AddressExtractor::extract_charm_holder_address(raw_hex, network)
        .ok()
        .flatten();

    // 6. Detect DEX operations + build tags
    let dex_result = dex::detect_dex_operation(&charm_json);
    let mut tag_list: Vec<String> = Vec::new();

    if let Some(ref result) = dex_result {
        tag_list.push("charms-cast".to_string());
        tag_list.push(result.operation.to_tag().to_string());
    }

    // Beaming detection
    let is_beaming = spell.tx.beamed_outs.is_some();
    if is_beaming {
        tag_list.push("beaming".to_string());
    }

    // $BRO token detection (check primary + all assets)
    if dex::is_bro_token(&app_id) {
        tag_list.push("bro".to_string());
    } else {
        for asset in &asset_infos {
            if dex::is_bro_token(&asset.app_id) {
                tag_list.push("bro".to_string());
                break;
            }
        }
    }

    let tags = if tag_list.is_empty() {
        None
    } else {
        Some(tag_list.join(","))
    };

    Some(AnalyzedTx {
        txid: txid.to_string(),
        charm_json,
        app_id,
        asset_type,
        amount,
        address,
        tags,
        dex_result,
        asset_infos,
        is_beaming,
        version: spell.version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_non_charm_tx() {
        // A random hex that is NOT a charm tx should return None
        let result = analyze_tx("abc123", "0200000001abcd", "mainnet");
        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_real_charm_from_file() {
        // Read a known charm tx hex from /tmp (same file used by native_charm_parser tests)
        let hex = match std::fs::read_to_string("/tmp/spell_tx.hex") {
            Ok(h) => h.trim().to_string(),
            Err(_) => return, // skip if file not present
        };
        let result = analyze_tx(
            "97dc8dd9d239a86efc0d7bf6154eb960001973d10d417b1f2bbb806771b2c26d",
            &hex,
            "mainnet",
        );
        assert!(result.is_some(), "should parse known charm tx");
        let analyzed = result.unwrap();
        assert_eq!(analyzed.version, 10);
        assert!(!analyzed.asset_infos.is_empty());
        assert!(analyzed.app_id.starts_with("t/") || analyzed.app_id.starts_with("n/"));
    }
}
