use charms_indexer::config::AppConfig;
use charms_indexer::domain::services::charm_detector::CharmDetectorService;
use charms_indexer::domain::services::CharmService;
use charms_indexer::infrastructure::api::client::ApiClient;
use charms_indexer::infrastructure::bitcoin::client::BitcoinClient;
use charms_indexer::infrastructure::persistence::repositories::charm_repository::CharmRepository;
use charms_indexer::infrastructure::persistence::DbPool;

#[tokio::test]
async fn test_api_call_with_known_charm_transaction() {
    // Test transaction that should contain a charm
    let test_txid = "507a50be2eed1b53af33fa903c09033ba105a3a6b58cb8e380940298335add0d";

    println!("Testing API call with transaction: {}", test_txid);

    // Load configuration
    let config = AppConfig::from_env();

    // Create API client
    let api_client = match ApiClient::new(&config) {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create API client: {}", e);
            panic!("Cannot create API client");
        }
    };

    // Test API call
    println!("Calling API...");
    match api_client.get_spell_data(test_txid).await {
        Ok(data) => {
            println!("✅ API Response received:");
            println!("{}", serde_json::to_string_pretty(&data).unwrap());

            // Verify the response contains expected charm data
            assert!(!data.is_null(), "API should return charm data");

            if let Some(obj) = data.as_object() {
                assert!(
                    obj.contains_key("version"),
                    "Response should contain version"
                );
                assert!(obj.contains_key("outs"), "Response should contain outs");

                if let Some(outs) = obj.get("outs").and_then(|v| v.as_array()) {
                    if let Some(first_out) = outs.first() {
                        if let Some(charms) = first_out.get("charms") {
                            println!("✅ Charm data found in API response");
                            assert!(charms.is_object(), "Charms should be an object");
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ API Error: {}", e);
            panic!("API call failed: {}", e);
        }
    }
}

#[test]
fn test_charm_detector_with_spell_marker() {
    // Test with a transaction that contains the "spell" marker
    let mock_tx_with_spell = hex::encode("some data spell more data");

    println!("Testing charm detection with mock transaction containing 'spell'");

    let could_be_charm = CharmDetectorService::could_be_charm(&mock_tx_with_spell);
    assert!(
        could_be_charm,
        "Transaction with 'spell' marker should be detected as potential charm"
    );

    let analysis = CharmDetectorService::analyze_charm_transaction(&mock_tx_with_spell);
    assert!(
        analysis.is_some(),
        "Analysis should succeed for transaction with 'spell' marker"
    );

    println!("✅ Charm detection working correctly with 'spell' marker");
}

#[test]
fn test_charm_detector_without_spell_marker() {
    // Test with a transaction that doesn't contain the "spell" marker
    let mock_tx_without_spell = hex::encode("some random transaction data");

    println!("Testing charm detection with mock transaction without 'spell'");

    let could_be_charm = CharmDetectorService::could_be_charm(&mock_tx_without_spell);
    assert!(
        !could_be_charm,
        "Transaction without 'spell' marker should not be detected as charm"
    );

    let analysis = CharmDetectorService::analyze_charm_transaction(&mock_tx_without_spell);
    assert!(
        analysis.is_none(),
        "Analysis should fail for transaction without 'spell' marker"
    );

    println!("✅ Charm detection correctly rejects transactions without 'spell' marker");
}

#[tokio::test]
#[ignore] // Run with: cargo test test_full_charm_processing_pipeline -- --ignored
async fn test_full_charm_processing_pipeline() {
    let test_txid = "507a50be2eed1b53af33fa903c09033ba105a3a6b58cb8e380940298335add0d";
    let config = AppConfig::from_env();

    // Create all required components
    let api_client = ApiClient::new(&config).expect("Failed to create API client");

    let bitcoin_client = match BitcoinClient::from_app_config(&config, "testnet4") {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create Bitcoin client: {}", e);
            return;
        }
    };

    let db_pool = match DbPool::new(&config).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("Failed to connect to database: {}", e);
            return;
        }
    };

    let charm_repository = CharmRepository::new(db_pool.get_connection().clone());
    let charm_service = CharmService::new(bitcoin_client, api_client, charm_repository);

    // Test the full pipeline
    let test_block_height = 100000u64;

    match charm_service
        .detect_and_process_charm(test_txid, test_block_height, None)
        .await
    {
        Ok(Some(charm)) => {
            println!("✅ Charm successfully detected and processed!");
            println!("Charm ID: {}", charm.charmid);
            println!("Transaction ID: {}", charm.txid);
            assert_eq!(charm.txid, test_txid);
            assert_eq!(charm.block_height, test_block_height);
        }
        Ok(None) => {
            panic!("❌ No charm detected - this suggests an issue with our detection logic");
        }
        Err(e) => {
            panic!("❌ Error processing charm: {}", e);
        }
    }
}
