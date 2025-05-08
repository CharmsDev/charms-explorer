// Web server implementation for the indexer

use axum::http::{header, Method};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::domain::services::DiagnosticService;
use crate::infrastructure::persistence::DbPool;

use super::routes::{create_router, AppState};

/// Start the web server
pub async fn start_server(
    config: &AppConfig,
    db_pool: &DbPool,
    bookmark_repository: crate::infrastructure::persistence::repositories::BookmarkRepository,
) {
    // Create diagnostic service
    let diagnostic_service = DiagnosticService::new(&db_pool.connection, bookmark_repository);

    // Create application state
    let state = AppState { diagnostic_service };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_origin(Any);

    // Create router with all routes
    let app = create_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Get port from config or use default
    let port = config.indexer.api_port.unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Start the server
    println!("Starting web server on http://localhost:{}", port);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
