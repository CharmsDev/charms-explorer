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

use axum::routing::{Router, delete, get, post};
use http::{Method, header};
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::ApiConfig;
use db::DbPool;
use services::mempool_space_service::EsploraClient;
use handlers::{
    AppState,
    broadcast_wallet_transaction, diagnose_database, diagnostics_address,
    get_asset_by_id, get_asset_counts,
    get_asset_holders, get_assets, get_charm_by_charmid, get_charm_by_txid, get_charm_numbers,
    get_charms, get_charms_by_address, get_charms_by_type, get_charms_count_by_type,
    get_all_orders, get_indexer_status, get_open_orders, get_order_by_id, get_orders_by_asset,
    get_orders_by_maker,
    get_reference_nft_by_hash, get_transaction_by_txid, get_transactions, get_wallet_balance,
    get_wallet_balance_batch,
    get_wallet_chain_tip, get_wallet_charm_balances, get_wallet_charm_balances_batch,
    get_wallet_charm_balances_batch_indexed,
    get_wallet_fee_estimate, get_wallet_prev_txs, get_wallet_transaction, get_wallet_transactions,
    get_wallet_transactions_batch,
    get_wallet_tx_hex, get_wallet_utxos, get_wallet_utxos_batch,
    health_check, like_charm, unlike_charm,
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

    // Initialize application state with repositories and config
    let repositories = db_pool.repositories();
    // Shared HTTP client for the Esplora (mempool.space) API:
    //   - pooled keep-alive connections to the single Esplora host
    //   - 10s connect timeout, 15s request timeout
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(64)
        .pool_idle_timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .tcp_keepalive(Duration::from_secs(60))
        // IPv6 from Fly to mempool.space hangs until the connect timeout.
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .user_agent(concat!("charms-explorer-api/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Failed to build HTTP client");
    tracing::info!(
        "Esplora: mainnet={} testnet4={}",
        config.bitcoin_mainnet_esplora_url,
        config.bitcoin_testnet4_esplora_url
    );

    let app_state = AppState {
        repositories: Arc::new(repositories),
        esplora: EsploraClient::new(
            http_client,
            &config.bitcoin_mainnet_esplora_url,
            &config.bitcoin_testnet4_esplora_url,
        ),
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
        .route(
            "/internal/diagnostics/address/{network}/{address}",
            get(diagnostics_address),
        )
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
        .route("/dex/orders", get(get_all_orders))
        .route("/dex/orders/open", get(get_open_orders))
        .route(
            "/dex/orders/by-asset/{asset_app_id}",
            get(get_orders_by_asset),
        )
        .route("/dex/orders/by-maker/{maker}", get(get_orders_by_maker))
        .route("/dex/orders/{order_id}", get(get_order_by_id))
        // Wallet
        .route("/wallet/utxos/{address}", get(get_wallet_utxos))
        .route("/wallet/utxos/batch", post(get_wallet_utxos_batch))
        .route("/wallet/balance/{address}", get(get_wallet_balance))
        .route("/wallet/balance/batch", post(get_wallet_balance_batch))
        .route("/wallet/tx/{txid}", get(get_wallet_transaction))
        .route("/wallet/tx/{txid}/hex", get(get_wallet_tx_hex))
        .route("/wallet/prev-txs", post(get_wallet_prev_txs))
        .route("/wallet/broadcast", post(broadcast_wallet_transaction))
        .route("/wallet/fee-estimate", get(get_wallet_fee_estimate))
        .route("/wallet/tip", get(get_wallet_chain_tip))
        .route(
            "/wallet/charms/batch",
            post(get_wallet_charm_balances_batch),
        )
        .route(
            "/wallet/charms/batch/indexed",
            post(get_wallet_charm_balances_batch_indexed),
        )
        .route("/wallet/charms/{address}", get(get_wallet_charm_balances))
        .route(
            "/wallet/transactions/{address}",
            get(get_wallet_transactions),
        )
        .route(
            "/wallet/transactions/batch",
            post(get_wallet_transactions_batch),
        );

    // Mount under /v1/ (canonical) and / (backward compat for Explorer webapp)
    let app = Router::new()
        .nest("/v1", api_routes.clone())
        .merge(api_routes)
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
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
