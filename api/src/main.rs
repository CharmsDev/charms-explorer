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

use axum::routing::{get, Router};
use http::{header, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::ApiConfig;
use db::DbPool;
use handlers::{
    get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_type, health_check,
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

    // Initialize data repositories
    let repositories = db_pool.repositories();
    let app_state = Arc::new(repositories);

    // Configure CORS policy
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
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
        .route("/charms/count", get(get_charm_numbers))
        .route("/charms/by-type", get(get_charms_by_type))
        .route("/charms/{txid}", get(get_charm_by_txid))
        .route("/charms", get(get_charms))
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
