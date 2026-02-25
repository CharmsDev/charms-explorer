// Charms Explorer API server entry point

mod config;
mod db;
mod entity;
mod error;
mod handlers;
mod models;
mod services;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::routing::{delete, get, post, Router};
use http::{header, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::ApiConfig;
use db::DbPool;
use handlers::{
    broadcast_wallet_transaction, diagnose_database, get_asset_by_id, get_asset_counts,
    get_asset_holders, get_assets, get_charm_by_charmid, get_charm_by_txid, get_charm_numbers,
    get_charms, get_charms_by_address, get_charms_by_type, get_charms_count_by_type,
    get_indexer_status, get_open_orders, get_order_by_id, get_orders_by_asset, get_orders_by_maker,
    get_reference_nft_by_hash, get_transaction_by_txid, get_transactions, get_wallet_balance,
    get_wallet_chain_tip, get_wallet_charm_balances, get_wallet_charm_balances_batch,
    get_wallet_fee_estimate, get_wallet_transaction, get_wallet_utxos, health_check, like_charm,
    unlike_charm, AppState,
};

fn load_env() {
    dotenv::dotenv().ok();
}

#[tokio::main]
async fn main() {
    load_env();
    // Configure logging with tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load API configuration from environment
    let config = ApiConfig::from_env();
    tracing::info!("Configuration loaded");

    // Establish database connection pool
    let db_pool = DbPool::new(&config)
        .await
        .expect("Failed to connect to database");
    tracing::info!("Connected to database");

    // Initialize shared Bitcoin RPC clients (one per network, reused across all requests)
    let rpc_mainnet = {
        let url = format!(
            "http://{}:{}",
            config.bitcoin_mainnet_rpc_host, config.bitcoin_mainnet_rpc_port
        );
        let auth = bitcoincore_rpc::Auth::UserPass(
            config.bitcoin_mainnet_rpc_username.clone(),
            config.bitcoin_mainnet_rpc_password.clone(),
        );
        Arc::new(
            bitcoincore_rpc::Client::new(&url, auth).expect("Failed to create mainnet RPC client"),
        )
    };
    let rpc_testnet4 = {
        let url = format!(
            "http://{}:{}",
            config.bitcoin_testnet4_rpc_host, config.bitcoin_testnet4_rpc_port
        );
        let auth = bitcoincore_rpc::Auth::UserPass(
            config.bitcoin_testnet4_rpc_username.clone(),
            config.bitcoin_testnet4_rpc_password.clone(),
        );
        Arc::new(
            bitcoincore_rpc::Client::new(&url, auth).expect("Failed to create testnet4 RPC client"),
        )
    };
    tracing::info!("Bitcoin RPC clients initialized (mainnet + testnet4)");

    // Initialize application state with repositories and config
    let repositories = db_pool.repositories();
    // HTTP client tuned for high-volume outbound calls (QuickNode, mempool.space)
    // Connection pool: 100 idle per host, 30s idle timeout, 10s connect timeout
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(100)
        .pool_idle_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .expect("Failed to build HTTP client");

    let app_state = AppState {
        repositories: Arc::new(repositories),
        config: config.clone(),
        scan_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
        quicknode_semaphore: Arc::new(tokio::sync::Semaphore::new(64)),
        http_client,
        rpc_mainnet,
        rpc_testnet4,
    };

    // Configure CORS policy
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::AUTHORIZATION,
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            header::ACCESS_CONTROL_REQUEST_METHOD,
        ])
        .expose_headers([header::CONTENT_TYPE, header::CONTENT_LENGTH])
        .max_age(Duration::from_secs(3600));

    // ── API routes (single definition, mounted at /v1/ and / for backward compat) ──
    let api_routes = Router::new()
        // Infrastructure
        .route("/health", get(health_check))
        .route("/status", get(get_indexer_status))
        .route("/diagnose", get(diagnose_database))
        // Charms
        .route("/charms", get(get_charms))
        .route("/charms/count", get(get_charm_numbers))
        .route("/charms/count-by-type", get(get_charms_count_by_type))
        .route("/charms/by-type", get(get_charms_by_type))
        .route("/charms/by-charmid/{charmid}", get(get_charm_by_charmid))
        .route("/charms/by-address/{address}", get(get_charms_by_address))
        .route("/charms/like", post(like_charm))
        .route("/charms/like", delete(unlike_charm))
        .route("/charms/{txid}", get(get_charm_by_txid))
        // Assets
        .route("/assets", get(get_assets))
        .route("/assets/count", get(get_asset_counts))
        .route(
            "/assets/reference-nft/{hash}",
            get(get_reference_nft_by_hash),
        )
        .route("/assets/{app_id}/holders", get(get_asset_holders))
        .route("/assets/{asset_id}", get(get_asset_by_id))
        // Transactions
        .route("/transactions", get(get_transactions))
        .route("/transactions/{txid}", get(get_transaction_by_txid))
        // DEX Orders
        .route("/dex/orders/open", get(get_open_orders))
        .route(
            "/dex/orders/by-asset/{asset_app_id}",
            get(get_orders_by_asset),
        )
        .route("/dex/orders/by-maker/{maker}", get(get_orders_by_maker))
        .route("/dex/orders/{order_id}", get(get_order_by_id))
        // Wallet
        .route("/wallet/utxos/{address}", get(get_wallet_utxos))
        .route("/wallet/balance/{address}", get(get_wallet_balance))
        .route("/wallet/tx/{txid}", get(get_wallet_transaction))
        .route("/wallet/broadcast", post(broadcast_wallet_transaction))
        .route("/wallet/fee-estimate", get(get_wallet_fee_estimate))
        .route("/wallet/tip", get(get_wallet_chain_tip))
        .route(
            "/wallet/charms/batch",
            post(get_wallet_charm_balances_batch),
        )
        .route("/wallet/charms/{address}", get(get_wallet_charm_balances));

    // Mount under /v1/ (canonical) and / (backward compat for Explorer webapp)
    let app = Router::new()
        .nest("/v1", api_routes.clone())
        .merge(api_routes)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(app_state);

    // Parse server address from config
    let addr: SocketAddr = config.server_addr().parse().expect("Invalid address");

    // Start HTTP server with high-concurrency Tokio settings
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
