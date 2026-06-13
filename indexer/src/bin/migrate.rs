//! `cargo run --bin migrate` — apply pending SQL migrations in order.
//!
//! Bundles every file in `../database/migrations/` into the binary at compile
//! time, so deployments do not need the source tree present at runtime. Each
//! file is applied inside a transaction and recorded in `seaql_migrations`.
//!
//! Versions are derived from filename prefix: `m{YYYYMMDD}_{NNNNNN}_*.sql`.
//! The full filename without `.sql` extension is used as the version key,
//! matching the convention already present in `seaql_migrations`.

use sea_orm::{ConnectionTrait, Database, DbBackend, Statement, TransactionTrait};

use charms_indexer::config::AppConfig;
use charms_indexer::utils::logging;

/// (filename_without_ext, sql_contents) — populated by `include_str!`.
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "m20260221_000001_mempool_indexing",
        include_str!("../../../database/migrations/m20260221_000001_mempool_indexing.sql"),
    ),
    (
        "m20260228_000001_address_utxos_and_backfill",
        include_str!(
            "../../../database/migrations/m20260228_000001_address_utxos_and_backfill.sql"
        ),
    ),
    (
        "m20260228_000002_address_transactions",
        include_str!("../../../database/migrations/m20260228_000002_address_transactions.sql"),
    ),
    (
        "m20260301_000001_transactions_mempool_columns",
        include_str!(
            "../../../database/migrations/m20260301_000001_transactions_mempool_columns.sql"
        ),
    ),
    (
        "m20260323_000001_tx_type_column",
        include_str!("../../../database/migrations/m20260323_000001_tx_type_column.sql"),
    ),
    (
        "m20260611_000001_stats_holders_network_column",
        include_str!(
            "../../../database/migrations/m20260611_000001_stats_holders_network_column.sql"
        ),
    ),
    (
        "m20260613_000001_mainnet_readiness",
        include_str!("../../../database/migrations/m20260613_000001_mainnet_readiness.sql"),
    ),
];

#[tokio::main]
async fn main() {
    logging::init_logger();

    let config = AppConfig::from_env();
    let conn = Database::connect(&config.database.url)
        .await
        .expect("connect to database");

    ensure_seaql_table(&conn).await;
    let applied = applied_versions(&conn).await;

    let mut applied_count = 0usize;
    for (version, sql) in MIGRATIONS {
        if applied.contains(*version) {
            logging::log_info(&format!("✓ already applied: {}", version));
            continue;
        }

        let tx = conn.begin().await.expect("begin transaction");
        if let Err(e) = tx
            .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
            .await
        {
            logging::log_error(&format!("✗ {} failed: {}", version, e));
            tx.rollback().await.ok();
            std::process::exit(1);
        }
        if let Err(e) = tx
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO seaql_migrations (version) VALUES ($1) ON CONFLICT (version) DO NOTHING",
                [(*version).into()],
            ))
            .await
        {
            logging::log_error(&format!("✗ {} record failed: {}", version, e));
            tx.rollback().await.ok();
            std::process::exit(1);
        }
        tx.commit().await.expect("commit transaction");

        logging::log_info(&format!("✓ applied: {}", version));
        applied_count += 1;
    }

    logging::log_info(&format!(
        "Migration run complete — applied {} new migration(s)",
        applied_count
    ));
}

async fn ensure_seaql_table(conn: &sea_orm::DatabaseConnection) {
    let sql = "CREATE TABLE IF NOT EXISTS seaql_migrations (\
                 version VARCHAR NOT NULL PRIMARY KEY, \
                 applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP\
               )";
    conn.execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await
        .expect("create seaql_migrations table");
}

async fn applied_versions(conn: &sea_orm::DatabaseConnection) -> std::collections::HashSet<String> {
    let rows = conn
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT version FROM seaql_migrations".to_string(),
        ))
        .await
        .expect("read seaql_migrations");
    rows.into_iter()
        .filter_map(|r| r.try_get::<String>("", "version").ok())
        .collect()
}
