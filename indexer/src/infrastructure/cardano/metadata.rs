//! Fetch Cardano token metadata via Koios API (free, no API key).
//! Reads CIP-68 inline datum from the reference NFT to extract name, symbol, image, decimals.

use crate::utils::logging;
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use bech32::{Bech32, Hrp};
use charms_client::cardano_tx;
use charms_data::App;
use cml_core::serialization::RawBytesEncoding;
use std::collections::HashMap;
use std::sync::Mutex;

static METADATA_CACHE: Mutex<Option<HashMap<String, Option<CardanoTokenMetadata>>>> =
    Mutex::new(None);

#[derive(Debug, Clone)]
pub struct CardanoTokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<u64>,
    pub policy_id: String,
    pub asset_name_hex: String,
    pub fingerprint: String,
}

/// Compute CIP-14 asset fingerprint: bech32("asset", blake2b_160(policy_id || asset_name))
pub fn compute_fingerprint(policy_id_hex: &str, asset_name_hex: &str) -> String {
    let combined = hex::decode(format!("{}{}", policy_id_hex, asset_name_hex))
        .unwrap_or_default();
    let mut hasher = Blake2bVar::new(20).expect("valid output size");
    hasher.update(&combined);
    let mut hash = [0u8; 20];
    hasher.finalize_variable(&mut hash).expect("correct length");
    let hrp = Hrp::parse("asset").expect("valid hrp");
    bech32::encode::<Bech32>(hrp, &hash).expect("valid bech32")
}

/// Derive Cardano policy_id and asset_name from a charms App struct.
/// PANICS if app.tag is not TOKEN ('t') or NFT ('n') — callers must filter.
pub fn derive_cardano_ids(app: &App) -> (String, String) {
    let (pid, _script) = cardano_tx::policy_id(app);
    let aname = cardano_tx::asset_name(app);
    (hex::encode(pid.to_raw_bytes()), hex::encode(aname.to_raw_bytes()))
}

/// Fetch metadata for a Cardano token. Uses Koios (free, no API key).
/// Returns None for non-token/nft apps (e.g. `c/` contracts have no Cardano mapping).
pub async fn fetch_metadata(app: &App) -> Option<CardanoTokenMetadata> {
    // Only TOKEN ('t') and NFT ('n') apps have Cardano policy_id/asset_name.
    // Contracts ('c') and other tags would panic in cardano_tx::asset_name.
    if app.tag != 't' && app.tag != 'n' {
        return None;
    }
    let (policy_id_hex, asset_name_hex) = derive_cardano_ids(app);
    let cache_key = format!("{}{}", policy_id_hex, asset_name_hex);

    // Check cache
    {
        let mut cache = METADATA_CACHE.lock().ok()?;
        let map = cache.get_or_insert_with(HashMap::new);
        if let Some(cached) = map.get(&cache_key) {
            return cached.clone();
        }
    }

    let result = fetch_cip68_metadata(&policy_id_hex, &asset_name_hex).await;

    // Cache result
    if let Ok(mut cache) = METADATA_CACHE.lock() {
        cache.get_or_insert_with(HashMap::new).insert(cache_key, result.clone());
    }

    result
}

/// Fetch CIP-68 metadata from the reference NFT's inline datum via Koios.
///
/// CIP-68 tokens have a companion reference NFT under the same policy_id.
/// The reference NFT's asset_name uses label 000643b0 (CIP-67 label 1)
/// instead of the token's label (0014df10 for FT, 000de140 for NFT).
/// The metadata lives in the UTXO's inline datum holding this reference NFT.
async fn fetch_cip68_metadata(
    policy_id_hex: &str,
    asset_name_hex: &str,
) -> Option<CardanoTokenMetadata> {
    // Derive the reference NFT asset_name: replace label with 000643b0
    let ref_asset_name = format!("000643b0{}", &asset_name_hex[8..]);

    let client = reqwest::Client::new();

    // Fetch the UTXO holding the reference NFT (includes inline datum)
    let body = serde_json::json!({
        "_asset_list": [[policy_id_hex, ref_asset_name]],
        "_extended": true
    });

    let response = match client
        .post("https://api.koios.rest/api/v1/asset_utxos")
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            logging::log_warning(&format!("Koios API error: {}", e));
            return None;
        }
    };

    let utxos: Vec<serde_json::Value> = match response.json().await {
        Ok(u) => u,
        Err(_) => return None,
    };

    let utxo = utxos.first()?;
    let datum = utxo.get("inline_datum")?;
    let datum_value = datum.get("value")?;

    // Parse CIP-68 datum structure: Constructor 0 [ metadata_map, version, extra ]
    let fields = datum_value.get("fields")?.as_array()?;
    let metadata_map = fields.first()?.get("map")?.as_array()?;

    let mut name = None;
    let mut symbol = None;
    let mut description = None;
    let mut image_url = None;
    let mut decimals = None;

    for entry in metadata_map {
        let key_hex = entry.get("k")?.get("bytes")?.as_str()?;
        let key = String::from_utf8(hex::decode(key_hex).ok()?).ok()?;

        match key.as_str() {
            "name" => {
                let v = entry.get("v")?.get("bytes")?.as_str()?;
                name = Some(String::from_utf8(hex::decode(v).ok()?).ok()?);
            }
            "ticker" => {
                let v = entry.get("v")?.get("bytes")?.as_str()?;
                symbol = Some(String::from_utf8(hex::decode(v).ok()?).ok()?);
            }
            "description" => {
                let v = entry.get("v")?.get("bytes")?.as_str()?;
                description = Some(String::from_utf8(hex::decode(v).ok()?).ok()?);
            }
            "logo" | "image" => {
                let v = entry.get("v")?.get("bytes")?.as_str()?;
                let raw = String::from_utf8(hex::decode(v).ok()?).ok()?;
                image_url = Some(if raw.starts_with("ipfs://") {
                    format!("https://ipfs.io/ipfs/{}", &raw[7..])
                } else {
                    raw
                });
            }
            "decimals" => {
                decimals = entry.get("v")?.get("int")?.as_u64().map(|d| d.min(18) as u8);
            }
            _ => {}
        }
    }

    // Also fetch total_supply from Koios asset_info
    let total_supply = fetch_total_supply(&client, policy_id_hex, asset_name_hex).await;

    logging::log_info(&format!(
        "Cardano CIP-68 metadata: name={:?} symbol={:?} decimals={:?} supply={:?}",
        name, symbol, decimals, total_supply
    ));

    let fingerprint = compute_fingerprint(policy_id_hex, asset_name_hex);

    Some(CardanoTokenMetadata {
        name,
        symbol,
        description,
        image_url,
        decimals,
        total_supply,
        policy_id: policy_id_hex.to_string(),
        asset_name_hex: asset_name_hex.to_string(),
        fingerprint,
    })
}

async fn fetch_total_supply(
    client: &reqwest::Client,
    policy_id_hex: &str,
    asset_name_hex: &str,
) -> Option<u64> {
    let url = format!(
        "https://api.koios.rest/api/v1/asset_info?_asset_policy={}&_asset_name={}",
        policy_id_hex, asset_name_hex
    );

    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;

    let data: Vec<serde_json::Value> = response.json().await.ok()?;
    let asset = data.first()?;
    asset.get("total_supply")?.as_str()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_cardano_ids() {
        let tokens = [
            ("BRO", "t/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f"),
            ("eBTC", "t/0796f63ed48144b4ec69fb794fbc2290ae63acf945fb035d5474648b50ee43b6/fd0cac892e457454be0212fa7d9a0e1517d5bd6a33aa7c66a1f10f55e375c290"),
        ];
        for (name, app_id) in tokens {
            let app: charms_data::App = app_id.parse().expect("parse");
            let (pid, aname) = derive_cardano_ids(&app);
            let fp = compute_fingerprint(&pid, &aname);
            println!("\n=== {} ===", name);
            println!("policy_id: {}", pid);
            println!("asset_name: {}", aname);
            println!("fingerprint: {}", fp);
        }
    }
}
