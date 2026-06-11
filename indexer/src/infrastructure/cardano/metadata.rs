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
use std::time::{Duration, Instant};

/// Negative-cache TTL: how long to remember that a fetch returned `None`
/// before retrying upstream. Positive results are cached permanently.
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(3600);

struct CacheEntry {
    value: Option<CardanoTokenMetadata>,
    /// `None` for positive entries (permanent); `Some(expiry)` for negatives.
    expires_at: Option<Instant>,
}

static METADATA_CACHE: Mutex<Option<HashMap<String, CacheEntry>>> = Mutex::new(None);

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

    {
        let mut cache = METADATA_CACHE.lock().ok()?;
        let map = cache.get_or_insert_with(HashMap::new);
        if let Some(entry) = map.get(&cache_key) {
            match entry.expires_at {
                None => return entry.value.clone(),                 // positive: permanent
                Some(exp) if Instant::now() < exp => return entry.value.clone(),
                _ => { map.remove(&cache_key); }                    // negative expired
            }
        }
    }

    let result = fetch_cip68_metadata(&policy_id_hex, &asset_name_hex).await;

    if let Ok(mut cache) = METADATA_CACHE.lock() {
        let map = cache.get_or_insert_with(HashMap::new);
        let entry = CacheEntry {
            expires_at: if result.is_none() {
                Some(Instant::now() + NEGATIVE_CACHE_TTL)
            } else {
                None
            },
            value: result.clone(),
        };
        map.insert(cache_key, entry);
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
    // Derive the reference NFT asset_name: replace the CIP-67 label (first 8
    // hex chars / 4 bytes) with 000643b0. Bail out if the name is too short to
    // carry a label — derive_cardano_ids always produces a full-length name
    // but external callers may not.
    if asset_name_hex.len() < 8 {
        logging::log_warning(&format!(
            "Cardano asset_name too short to derive reference NFT: '{}'",
            asset_name_hex
        ));
        return None;
    }
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

    const BRO_APP_ID: &str = "t/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f";
    const BRO_POLICY_ID: &str = "b8f72e95dee612df98ac5a90b7604f7815c2af07a6db209a5c70abe4";
    const BRO_ASSET_NAME: &str =
        "0014df10cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b";

    #[test]
    fn derive_cardano_ids_for_bro_token() {
        let app: App = BRO_APP_ID.parse().expect("parse BRO app_id");
        let (pid, aname) = derive_cardano_ids(&app);
        assert_eq!(pid, BRO_POLICY_ID);
        assert_eq!(aname, BRO_ASSET_NAME);
    }

    #[test]
    fn compute_fingerprint_is_deterministic_for_bro() {
        let fp1 = compute_fingerprint(BRO_POLICY_ID, BRO_ASSET_NAME);
        let fp2 = compute_fingerprint(BRO_POLICY_ID, BRO_ASSET_NAME);
        assert_eq!(fp1, fp2);
        assert!(fp1.starts_with("asset1"), "got: {fp1}");
    }

    #[test]
    fn compute_fingerprint_changes_with_inputs() {
        let fp_bro = compute_fingerprint(BRO_POLICY_ID, BRO_ASSET_NAME);
        let other_policy = "0000000000000000000000000000000000000000000000000000000000000000";
        let fp_other = compute_fingerprint(other_policy, BRO_ASSET_NAME);
        assert_ne!(fp_bro, fp_other);
    }

    /// Guards the invariant that `derive_cardano_ids` produces a full-length
    /// asset name (≥ 8 hex chars / 4 bytes). The CIP-68 reference NFT
    /// derivation in `fetch_cip68_metadata` now bails out gracefully on
    /// short names, but real apps never produce one.
    #[test]
    fn derive_cardano_ids_yields_asset_name_at_least_8_chars() {
        let app: App = BRO_APP_ID.parse().unwrap();
        let (_, aname) = derive_cardano_ids(&app);
        assert!(aname.len() >= 8, "asset_name too short: {aname}");
    }
}
