// Indexer reset endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::{json, Value};

use crate::handlers::AppState;

/// Handler for POST /reset - Resets the indexer state
pub async fn reset_indexer(State(app_state): State<AppState>) -> impl IntoResponse {
    let conn = app_state.repositories.charm.get_connection();
    let result = perform_reset(conn).await;
    Json(result)
}

/// Performs the reset operation by clearing all indexer tables
async fn perform_reset(conn: &DatabaseConnection) -> Value {
    // Clear all tables
    let bookmark_result = clear_table(conn, "bookmark").await;
    let transactions_result = clear_table(conn, "transactions").await;
    let charms_result = clear_table(conn, "charms").await;

    // Check if all operations were successful
    let success = bookmark_result.is_ok() && transactions_result.is_ok() && charms_result.is_ok();

    // Collect any error messages
    let mut errors = Vec::new();
    if let Err(e) = &bookmark_result {
        errors.push(format!("Failed to clear bookmark table: {}", e));
    }
    if let Err(e) = &transactions_result {
        errors.push(format!("Failed to clear transactions table: {}", e));
    }
    if let Err(e) = &charms_result {
        errors.push(format!("Failed to clear charms table: {}", e));
    }

    json!({
        "success": success,
        "message": if success {
            "Indexer has been reset. All tables have been cleared. Restart the indexer service to begin indexing from the beginning.".to_string()
        } else {
            format!("Failed to reset indexer: {}", errors.join(", "))
        },
        "tables_cleared": {
            "bookmark": bookmark_result.is_ok(),
            "transactions": transactions_result.is_ok(),
            "charms": charms_result.is_ok()
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
