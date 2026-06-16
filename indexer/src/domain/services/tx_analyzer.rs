//! Unified transaction analyzer — single source of truth for charm detection.
//!
//! All code paths (block processing, mempool) use this module to parse a
//! raw transaction hex and extract charm/spell/DEX data.
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
    pub tx_type: String,
    /// Output indices present in spell.tx.beamed_outs — tokens at these outputs
    /// are committed to Cardano and must be reported with amount=0 on Bitcoin.
    pub beamed_out_indices: std::collections::HashSet<usize>,
}

/// Pure analysis: parse raw tx hex → Option<AnalyzedTx>.
/// Returns None if the tx does not contain a charm spell.
/// This is intentionally a free function, not a method on a struct,
/// because it needs no state — only the raw bytes and the network name.
/// Two verification modes (Plan 15):
///   * `Strict` — used by the block path. Demands a valid ZK proof against
///     the canonical VK chain (V0..V_CURRENT). A failure here MUST keep
///     the spell out of confirmed tables.
///   * `Permissive` — used by the mempool path. Parses the spell structure
///     without verifying its proof so the explorer can show pending
///     activity in real time. Anything written by the permissive path
///     carries the "pending" markers (`block_height IS NULL` / `= 0`,
///     `status='pending'`) and never touches `stats_holders` / `assets` /
///     confirmed rows. On block confirmation, the block path re-evaluates
///     in Strict mode and the consolidator either promotes the row or
///     purges it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode {
    Strict,
    Permissive,
}

pub fn analyze_tx(
    txid: &str,
    raw_hex: &str,
    network: &str,
    mode: VerifyMode,
) -> Option<AnalyzedTx> {
    let spell = match mode {
        VerifyMode::Strict => NativeCharmParser::extract_and_verify_charm(raw_hex, false).ok()?,
        VerifyMode::Permissive => NativeCharmParser::extract_spell_no_verify(raw_hex).ok()?,
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

    // Beaming detection:
    // - beam-out: spell has beamed_outs (sending tokens TO Cardano)
    // - beam-in/cross-chain: tx involves a c/ (contract) app alongside t/ tokens
    //   This covers beam-in (receiving from Cardano) and cross-chain operations
    let has_beamed_outs = spell.tx.beamed_outs.is_some();
    let beamed_out_indices: std::collections::HashSet<usize> = spell_json
        .get("tx")
        .and_then(|tx| tx.get("beamed_outs"))
        .and_then(|bo| bo.as_object())
        .map(|obj| obj.keys().filter_map(|k| k.parse::<usize>().ok()).collect())
        .unwrap_or_default();
    let has_contract_app = spell
        .app_public_inputs
        .keys()
        .any(|app| app.to_string().starts_with("c/"));
    let is_beaming = has_beamed_outs || has_contract_app;
    if has_beamed_outs {
        tag_list.push("beaming".to_string());
        tag_list.push("beam-out".to_string());
    } else if has_contract_app {
        tag_list.push("beaming".to_string());
        tag_list.push("beam-in".to_string());
    }

    // $BRO token detection + classify mint vs transfer
    // Mint: outputs have more charms than inputs (new tokens created from Bitcoin)
    // Transfer: outputs redistribute existing charms (total unchanged)
    // Detection: count how many spell outputs carry this token. If only 1 output
    // and no other token outputs exist, it's likely a mint. Multi-output = transfer.
    let is_bro = dex::is_bro_token(&app_id)
        || asset_infos.iter().any(|a| dex::is_bro_token(&a.app_id));
    if is_bro {
        tag_list.push("bro".to_string());
        let bro_output_count = asset_infos
            .iter()
            .filter(|a| dex::is_bro_token(&a.app_id))
            .count();
        // Single BRO output = mint (new tokens). Multiple BRO outputs = transfer (dest + change).
        if bro_output_count <= 1 {
            tag_list.push("bro-mint".to_string());
        } else {
            tag_list.push("bro-transfer".to_string());
        }
    }

    // eBTC token detection (check primary + all assets)
    if dex::is_ebtc_token(&app_id) {
        tag_list.push("ebtc".to_string());
    } else {
        for asset in &asset_infos {
            if dex::is_ebtc_token(&asset.app_id) {
                tag_list.push("ebtc".to_string());
                break;
            }
        }
    }

    let tags = if tag_list.is_empty() {
        None
    } else {
        Some(tag_list.join(","))
    };

    // Compute tx_type from tags (single source of truth)
    let tx_type = if tag_list.iter().any(|t| t == "create-ask") {
        "dex_create_ask"
    } else if tag_list.iter().any(|t| t == "create-bid") {
        "dex_create_bid"
    } else if tag_list.iter().any(|t| t == "fulfill-ask") {
        "dex_fulfill_ask"
    } else if tag_list.iter().any(|t| t == "fulfill-bid") {
        "dex_fulfill_bid"
    } else if tag_list.iter().any(|t| t == "cancel") {
        "dex_cancel"
    } else if tag_list.iter().any(|t| t == "partial-fill") {
        "dex_partial_fill"
    } else if has_beamed_outs {
        "beam_out"
    } else if has_contract_app {
        "beam_in"
    } else if tag_list.iter().any(|t| t == "bro-mint") {
        "bro_mint"
    } else if tag_list.iter().any(|t| t == "bro-transfer") {
        "token_transfer"
    } else {
        "spell"
    }
    .to_string();

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
        tx_type,
        beamed_out_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_non_charm_tx() {
        // A random hex that is NOT a charm tx should return None in both modes
        assert!(analyze_tx("abc123", "0200000001abcd", "mainnet", VerifyMode::Strict).is_none());
        assert!(
            analyze_tx("abc123", "0200000001abcd", "mainnet", VerifyMode::Permissive).is_none()
        );
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
            VerifyMode::Strict,
        );
        assert!(result.is_some(), "should parse known charm tx");
        let analyzed = result.unwrap();
        assert_eq!(analyzed.version, 10);
        assert!(!analyzed.asset_infos.is_empty());
        assert!(analyzed.app_id.starts_with("t/") || analyzed.app_id.starts_with("n/"));
    }
}
