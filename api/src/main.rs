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
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::ApiConfig;
use db::DbPool;
use handlers::{
    diagnose_database, get_asset_by_id, get_asset_counts, get_asset_holders, get_assets, get_charm_by_charmid, get_charm_by_txid, get_charm_numbers, get_charms,
    get_charms_by_address, get_charms_by_type, get_indexer_status, health_check, like_charm, reset_indexer, unlike_charm, AppState,
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
    let app_state = AppState {
        repositories: Arc::new(repositories),
        config: config.clone(),
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

    // Set up API routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(get_indexer_status))
        .route("/diagnose", get(diagnose_database))
        .route("/reset", post(reset_indexer))
        .route("/charms/count", get(get_charm_numbers))
        .route("/charms/by-type", get(get_charms_by_type))
        .route("/charms/by-charmid/{charmid}", get(get_charm_by_charmid))
        .route("/charms/by-address/{address}", get(get_charms_by_address))
        .route("/charms/like", post(like_charm))
        .route("/charms/like", delete(unlike_charm))
        .route("/charms/{txid}", get(get_charm_by_txid))
        .route("/charms", get(get_charms))
        .route("/assets/count", get(get_asset_counts))
        .route("/assets/{app_id}/holders", get(get_asset_holders))
        .route("/assets/{asset_id}", get(get_asset_by_id))
        .route("/assets", get(get_assets))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Parse server address from config
    let addr: SocketAddr = config.server_addr().parse().expect("Invalid address");

    // Start HTTP server
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
