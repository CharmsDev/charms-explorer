use anyhow::Result;
use charms_client::tx::{EnchantedTx, Tx};
use charms_client::NormalizedSpell;
// Import V9_SPELL_VK from charms_client - the library handles version detection internally
// For versions V0-V9, the library uses its internal VKs automatically
// For CURRENT_VERSION (V10+), we pass V9_SPELL_VK as fallback (will be updated when V10 VK is published)
use charms_client::V9_SPELL_VK;

/// Native charm parser using the charms-client crate
/// Provides direct parsing and verification of charm transactions
pub struct NativeCharmParser;

impl NativeCharmParser {
    /// Spell verification key - uses V9_SPELL_VK from charms_client
    /// The library internally handles version detection and uses the correct VK for each version
    pub const SPELL_VK: &'static str = V9_SPELL_VK;

    /// Extract and verify a charm from a transaction hex string
    ///
    /// This function:
    /// 1. Parses the transaction hex into a Tx object
    /// 2. Extracts the spell data from the transaction
    /// 3. Verifies the cryptographic proof
    /// 4. Returns the normalized spell data
    ///
    /// Returns Ok(NormalizedSpell) if the transaction contains a valid charm
    /// Returns Err if the transaction is invalid or doesn't contain a charm
    pub fn extract_and_verify_charm(tx_hex: &str, mock: bool) -> Result<NormalizedSpell> {
        // Parse transaction from hex using TryFrom
        let tx: Tx = tx_hex.try_into()?;

        // Extract and verify spell using the EnchantedTx trait method
        let normalized_spell = tx.extract_and_verify_spell(Self::SPELL_VK, mock)?;

        Ok(normalized_spell)
    }

    /// Check if a transaction hex could potentially be a charm
    /// This is a lightweight check before doing full parsing
    pub fn could_be_charm(tx_hex: &str) -> bool {
        // Try to parse as transaction first using TryFrom
        if let Ok(_tx) = Tx::try_from(tx_hex) {
            // Try to extract spell - if it succeeds, it's a charm
            Self::extract_and_verify_charm(tx_hex, false).is_ok()
        } else {
            false
        }
    }

    /// Extract asset-related data from a normalized spell
    /// Returns information that can be used to populate the assets table
    pub fn extract_asset_info(spell: &NormalizedSpell) -> Vec<AssetInfo> {
        let mut assets = Vec::new();

        // Extract information from spell outputs (NormalizedCharms)
        for (output_index, normalized_charms) in spell.tx.outs.iter().enumerate() {
            // Each output can contain multiple charms
            for (app_index, charm_data) in normalized_charms.iter() {
                // Get the app from the spell's app_public_inputs
                if let Some((app, _)) = spell.app_public_inputs.iter().nth(*app_index as usize) {
                    let asset_info = AssetInfo {
                        // [RJJ-FIX] app_id is directly from App.to_string(): {tag}/{identity}/{vk}
                        // NO output_index - that was incorrect
                        app_id: app.to_string(),
                        vout_index: output_index as i32,
                        amount: extract_amount_from_charm_data(charm_data),
                        asset_type: determine_asset_type_from_app(app),
                    };
                    assets.push(asset_info);
                }
            }
        }

        assets
    }
}

/// Information extracted from a charm that's relevant for the assets table
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub app_id: String,
    pub vout_index: i32,
    pub amount: u64,
    pub asset_type: String,
}

/// Extract amount from charm data
fn extract_amount_from_charm_data(charm_data: &charms_data::Data) -> u64 {
    // Try to extract amount from charm data as u64
    // This is a simplified implementation - can be enhanced based on actual data structure
    if let Ok(amount) = charm_data.value::<u64>() {
        // Cap at i64::MAX to prevent overflow when storing in database
        amount.min(i64::MAX as u64)
    } else if let Ok(amount) = charm_data.value::<i64>() {
        amount.max(0) as u64
    } else {
        // Try to extract from bytes representation
        let bytes = charm_data.bytes();
        if bytes.len() >= 8 {
            let raw_amount = u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            // Cap at i64::MAX to prevent overflow when storing in database
            raw_amount.min(i64::MAX as u64)
        } else {
            0
        }
    }
}

/// Determine the asset type based on app information
fn determine_asset_type_from_app(app: &charms_data::App) -> String {
    // Logic to determine asset type based on app tag
    // This follows the charm protocol rules for asset types
    match app.tag {
        charms_data::TOKEN => "token".to_string(),
        charms_data::NFT => "nft".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_vk_constant() {
        assert!(!NativeCharmParser::SPELL_VK.is_empty());
        assert!(NativeCharmParser::SPELL_VK.starts_with("0x"));
    }

    #[test]
    fn test_could_be_charm_invalid_hex() {
        let invalid_hex = "invalid_hex_string";
        assert!(!NativeCharmParser::could_be_charm(invalid_hex));
    }

    #[test]
    fn test_could_be_charm_regular_transaction() {
        // Regular Bitcoin transaction without charm data
        let regular_tx = "0200000001f2b3eb2deb76566e7324307cd47c35eeb88413f971d88519859b1834307ecfec01000000006a47304402203ad1cc746a3cb70ca10e7e3612a9370b8b5d4c8b3b5c7b5c7b5c7b5c7b5c7b5c02203ad1cc746a3cb70ca10e7e3612a9370b8b5d4c8b3b5c7b5c7b5c7b5c7b5c7b5c0121025476c2e83188368da1ff3e292e7acafcdb3566bb0ad253f62fc70f07aeee6357ffffffff0100e1f50500000000196a17a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba8700000000";
        assert!(!NativeCharmParser::could_be_charm(regular_tx));
    }

    /// Test that simulates exactly what the indexer does:
    /// 1. Deserialize hex with bitcoin::consensus (as if from block)
    /// 2. Re-serialize with bitcoin::consensus::encode::serialize_hex
    /// 3. Try to parse with NativeCharmParser
    /// This catches any roundtrip serialization issues.
    #[test]
    fn test_beaming_tx_roundtrip_like_indexer() {
        // Real beaming tx hex from mempool.space
        let original_hex = "0200000000010236c4d581d18b974562cf32e835c2c1e5d2590cd7c070e5ece35ebcb672216ae00000000000ffffffff36c4d581d18b974562cf32e835c2c1e5d2590cd7c070e5ece35ebcb672216ae00400000000ffffffff042302000000000000160014b9698eae2e61774bd88749e0f9b169acc50f99fb23020000000000001600144b1f883dd56c33fb7300dc27fc125eab88a462cf0000000000000000fd01036a057370656c6c4df70282a36776657273696f6e09627478a2646f75747382a1001a05f5e100a1001b00000001954fc4006b6265616d65645f6f757473a1009820189b18d30f18460718be1833181b18bc18a518b318ae18fa18bc18c9185b18b1185618c2185218e618a912182718d318cb18ac182a18b6184418ab189e716170705f7075626c69635f696e70757473a18361749820183d187f18e718e418ce18a6121819184718af187318d70e1851181918be18bd188a18a518b718ed18fe187418bf18af186e1877189a1818184718bd189b982018c9187518d418e018c2189218fb189518ef18bd18a518c118331218d618ac181d188b185a18ef18f718f018f118e518571886184518a218da187018ff185ff699010418a41859184c185909186c18de186e18ca18a818a618b81318201841185118df1827183718ad18ed18501866185418f1182f18e41861185c08182d185318da182418dc1883181a187918b018c6183c0f02185218c416186818c918a518c91865182118a618af18fc184018e218ba18f518cd181b1318dc18a0184d184a18e418da0b188c18c0184718691835185f18e018b218ce18ba189b18fe188118d918d4185b1845183b18e5188218b818f71865182f0218bb185318330e187518581819181909182d18cd18271898188b182f18d018571881183b18e218d818e918b4185418ed184c1832186e091218e1030918fd18e7187d187c0d1820187b18f5182118621834182818ff183b187c18c71867186b183818631876182618d918e218a018d918ad121893181d18f5181918a913184e183700171818182d189e18b118520418b1187718a318e818ea1882184118fd183818830e181a18b3184018cf18ab18d7185c18a218e3182e1825183718fb18a50b184718b6185b184e001866188e184c1898188518b518a8183e189618e0185d15186718c8185c18e3188618cf18f6189718b418d5184f186618df040d00183f18351855188918c1182b18c9181d18dc0418d31882186114181b188817181b185018e318d618ae1847182116186b18b618d8185f1877db0e000000000000160014542e0cd4e07fa3d919cf5aa5f8242612d4a3b3550247304402202bd582e27041fc7ae426853826e9c52cf8a5c8e5755026a6ba21892bdbcf51cb02201fef093148d7e5fa8ad7a10c8f953398db4a7efca4003d6a5c7cbbb58e468d84012103d5453d402d158c84de22dd20caf3cc1968178b5f674f5ec6d063d9fe8675fb27024730440220344cf2c2812b1c086c931bb6c1643bc91f33f4347011c594c37f54c4b23ea16a022049fad130880b7e88fc20e267cf151e23d32ed2669d94937e066658defea8c97b012103d5453d402d158c84de22dd20caf3cc1968178b5f674f5ec6d063d9fe8675fb2700000000";

        // Step 1: Deserialize with bitcoin::consensus (same as indexer receiving from node)
        let bytes = hex::decode(original_hex).expect("Failed to decode hex");
        let tx: bitcoin::Transaction =
            bitcoin::consensus::encode::deserialize(&bytes).expect("Failed to deserialize tx");

        // Step 2: Re-serialize (same as extract_transaction_data in block_processor.rs)
        let reserialized_hex = bitcoin::consensus::encode::serialize_hex(&tx);

        println!("Original hex length: {}", original_hex.len());
        println!("Reserialized hex length: {}", reserialized_hex.len());
        println!("Hex match: {}", original_hex == reserialized_hex);

        if original_hex != reserialized_hex {
            // Find first difference
            for (i, (a, b)) in original_hex
                .chars()
                .zip(reserialized_hex.chars())
                .enumerate()
            {
                if a != b {
                    println!(
                        "FIRST DIFF at pos {}: original='{}' reserialized='{}'",
                        i, a, b
                    );
                    println!(
                        "  original context:     ...{}...",
                        &original_hex
                            [i.saturating_sub(10)..std::cmp::min(i + 10, original_hex.len())]
                    );
                    println!(
                        "  reserialized context: ...{}...",
                        &reserialized_hex
                            [i.saturating_sub(10)..std::cmp::min(i + 10, reserialized_hex.len())]
                    );
                    break;
                }
            }
        }

        // Step 3: Try to parse the RESERIALIZED hex (this is what the indexer does)
        let result = NativeCharmParser::extract_and_verify_charm(&reserialized_hex, false);
        println!("Parse reserialized hex (mock=false): {}", result.is_ok());
        if let Err(ref e) = result {
            println!("ERROR parsing reserialized: {:?}", e);
        }

        // Step 4: Also try original hex for comparison
        let orig_result = NativeCharmParser::extract_and_verify_charm(original_hex, false);
        println!("Parse original hex (mock=false): {}", orig_result.is_ok());
        if let Err(ref e) = orig_result {
            println!("ERROR parsing original: {:?}", e);
        }

        // Both should succeed
        assert!(orig_result.is_ok(), "Original hex should parse as charm");
        assert!(result.is_ok(), "Reserialized hex should parse as charm");

        // Print the full charm JSON (same as what detection.rs produces)
        if let Ok(spell) = result {
            let charm_json = serde_json::to_value(&spell).unwrap();
            let full_data = serde_json::json!({
                "type": "spell",
                "detected": true,
                "has_native_data": true,
                "native_data": charm_json,
                "version": "native_parser"
            });
            println!("CHARM_JSON_START");
            println!("{}", serde_json::to_string(&full_data).unwrap());
            println!("CHARM_JSON_END");
        }
    }

    #[test]
    fn test_beaming_tx_parsing() {
        // Beaming transaction 8d70833ad1ce5d84cffb76fdc6038d669c6cf1808f3f84f3f0d83cad712e33a3
        // OP_RETURN spell is at vout[2], not vout[0]
        let beaming_tx_hex = "0200000000010236c4d581d18b974562cf32e835c2c1e5d2590cd7c070e5ece35ebcb672216ae00000000000ffffffff36c4d581d18b974562cf32e835c2c1e5d2590cd7c070e5ece35ebcb672216ae00400000000ffffffff042302000000000000160014b9698eae2e61774bd88749e0f9b169acc50f99fb23020000000000001600144b1f883dd56c33fb7300dc27fc125eab88a462cf0000000000000000fd01036a057370656c6c4df70282a36776657273696f6e09627478a2646f75747382a1001a05f5e100a1001b00000001954fc4006b6265616d65645f6f757473a1009820189b18d30f18460718be1833181b18bc18a518b318ae18fa18bc18c9185b18b1185618c2185218e618a912182718d318cb18ac182a18b6184418ab189e716170705f7075626c69635f696e70757473a18361749820183d187f18e718e418ce18a6121819184718af187318d70e1851181918be18bd188a18a518b718ed18fe187418bf18af186e1877189a1818184718bd189b982018c9187518d418e018c2189218fb189518ef18bd18a518c118331218d618ac181d188b185a18ef18f718f018f118e518571886184518a218da187018ff185ff699010418a41859184c185909186c18de186e18ca18a818a618b81318201841185118df1827183718ad18ed18501866185418f1182f18e41861185c08182d185318da182418dc1883181a187918b018c6183c0f02185218c416186818c918a518c91865182118a618af18fc184018e218ba18f518cd181b1318dc18a0184d184a18e418da0b188c18c0184718691835185f18e018b218ce18ba189b18fe188118d918d4185b1845183b18e5188218b818f71865182f0218bb185318330e187518581819181909182d18cd18271898188b182f18d018571881183b18e218d818e918b4185418ed184c1832186e091218e1030918fd18e7187d187c0d1820187b18f5182118621834182818ff183b187c18c71867186b183818631876182618d918e218a018d918ad121893181d18f5181918a913184e183700171818182d189e18b118520418b1187718a318e818ea1882184118fd183818830e181a18b3184018cf18ab18d7185c18a218e3182e1825183718fb18a50b184718b6185b184e001866188e184c1898188518b518a8183e189618e0185d15186718c8185c18e3188618cf18f6189718b418d5184f186618df040d00183f18351855188918c1182b18c9181d18dc0418d31882186114181b188817181b185018e318d618ae1847182116186b18b618d8185f1877db0e000000000000160014542e0cd4e07fa3d919cf5aa5f8242612d4a3b3550247304402202bd582e27041fc7ae426853826e9c52cf8a5c8e5755026a6ba21892bdbcf51cb02201fef093148d7e5fa8ad7a10c8f953398db4a7efca4003d6a5c7cbbb58e468d84012103d5453d402d158c84de22dd20caf3cc1968178b5f674f5ec6d063d9fe8675fb27024730440220344cf2c2812b1c086c931bb6c1643bc91f33f4347011c594c37f54c4b23ea16a022049fad130880b7e88fc20e267cf151e23d32ed2669d94937e066658defea8c97b012103d5453d402d158c84de22dd20caf3cc1968178b5f674f5ec6d063d9fe8675fb2700000000";

        println!("Testing beaming tx parsing...");

        // Test 1: Can the library parse this as a Bitcoin tx?
        let tx_result = Tx::try_from(beaming_tx_hex);
        println!("Tx parse result: {:?}", tx_result.is_ok());
        assert!(tx_result.is_ok(), "Should parse as valid Bitcoin tx");

        // Test 2: Can the library extract and verify the spell?
        let result = NativeCharmParser::extract_and_verify_charm(beaming_tx_hex, false);
        println!("extract_and_verify_charm result: {:?}", result.is_ok());
        if let Err(ref e) = result {
            println!("Error: {:?}", e);
        }

        // Test 3: Try with mock=true to skip proof verification
        let mock_result = NativeCharmParser::extract_and_verify_charm(beaming_tx_hex, true);
        println!(
            "extract_and_verify_charm (mock) result: {:?}",
            mock_result.is_ok()
        );
        if let Err(ref e) = mock_result {
            println!("Mock error: {:?}", e);
        }
        if let Ok(ref spell) = mock_result {
            println!("Spell version: {}", spell.version);
            println!("Spell outs count: {}", spell.tx.outs.len());
            println!("Spell has beamed_outs: {}", spell.tx.beamed_outs.is_some());
            let assets = NativeCharmParser::extract_asset_info(spell);
            println!("Assets found: {}", assets.len());
            for a in &assets {
                println!(
                    "  Asset: app_id={}, vout_index={}, amount={}, type={}",
                    a.app_id, a.vout_index, a.amount, a.asset_type
                );
            }
        }
    }
}
