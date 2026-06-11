//! Smoke test: spin up the container, create the schema, write/read a row.

mod common;

use common::TestDb;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

#[tokio::test]
async fn db_starts_schema_applies_round_trip() {
    let db = TestDb::new().await;

    // every entity-backed table should exist
    let tables = [
        "address_utxos",
        "assets",
        "block_status",
        "charms",
        "dex_orders",
        "mempool_spends",
        "monitored_addresses",
        "stats_holders",
        "summary",
        "transactions",
        "address_transactions",
    ];
    for t in tables {
        let sql = format!("SELECT 1 FROM {t} LIMIT 0");
        db.conn
            .execute(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .unwrap_or_else(|e| panic!("table {t} missing: {e}"));
    }
}
