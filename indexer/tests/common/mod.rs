//! Shared test infrastructure.
//!
//! Starts an ephemeral Postgres container per `TestDb::new()` and provisions
//! the schema from `tests/fixtures/schema.sql` (kept in sync with the entity
//! definitions). Containers are auto-removed by testcontainers when the
//! handle is dropped, even on panic.

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres as PostgresImage;

pub struct TestDb {
    #[allow(dead_code)] // kept alive so the container stays running for the test
    container: ContainerAsync<PostgresImage>,
    pub conn: DatabaseConnection,
}

impl TestDb {
    pub async fn new() -> Self {
        let container = PostgresImage::default()
            .start()
            .await
            .expect("start postgres");
        let host = container.get_host().await.expect("host");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped port");
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

        let conn = Database::connect(&url).await.expect("connect");
        apply_schema(&conn).await;
        Self { container, conn }
    }
}

const SCHEMA: &str = include_str!("../fixtures/schema.sql");

async fn apply_schema(conn: &DatabaseConnection) {
    for raw in SCHEMA.split(';') {
        let stmt = raw.trim();
        if stmt.is_empty() {
            continue;
        }
        conn.execute(Statement::from_string(DbBackend::Postgres, stmt.to_string()))
            .await
            .unwrap_or_else(|e| panic!("apply schema (stmt: {stmt:.60}…): {e}"));
    }
}
