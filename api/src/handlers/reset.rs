// Indexer reset endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::db::repositories::Repositories;

/// Handler for POST /reset - Resets the indexer state
pub async fn reset_indexer(State(repositories): State<Arc<Repositories>>) -> impl IntoResponse {
    let conn = repositories.charm.get_connection();
    let result = perform_reset(conn).await;
    Json(result)
}

/// Performs the reset operation by clearing the bookmark table
async fn perform_reset(conn: &DatabaseConnection) -> Value {
    // Clear the bookmark table
    let bookmark_result = clear_table(conn, "bookmark").await;

    // Optionally, we could also clear the transactions and charms tables
    // But we'll leave that as a separate operation for now
    // let transactions_result = clear_table(conn, "transactions").await;
    // let charms_result = clear_table(conn, "charms").await;

    json!({
        "success": bookmark_result.is_ok(),
        "message": if bookmark_result.is_ok() {
            "Indexer has been reset. Restart the indexer service to begin indexing from the beginning.".to_string()
        } else {
            format!("Failed to reset indexer: {}", bookmark_result.unwrap_err())
        }
    })
}

/// Clears all rows from a table
async fn clear_table(conn: &DatabaseConnection, table: &str) -> Result<(), String> {
    let query = format!("DELETE FROM {}", table);

    match conn
        .execute(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to clear table {}: {}", table, e)),
    }
}
