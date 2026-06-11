//! Integration tests for `StatsHoldersRepository`.
//!
//! Covers the original audit findings:
//!   N1 — non-idempotent re-processing (still open; documented as known bug)
//!   N2 — cross-network leak (FIXED in T3.3, regression test below)

mod common;

use charms_indexer::infrastructure::persistence::repositories::StatsHoldersRepository;
use common::TestDb;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

async fn row(
    conn: &sea_orm::DatabaseConnection,
    app_id: &str,
    address: &str,
    network: &str,
) -> Option<(i64, i32)> {
    let sql = format!(
        "SELECT total_amount, charm_count FROM stats_holders \
         WHERE app_id = '{}' AND address = '{}' AND network = '{}'",
        app_id, address, network
    );
    let res = conn
        .query_one(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap()?;
    let amt: i64 = res.try_get("", "total_amount").unwrap();
    let cnt: i32 = res.try_get("", "charm_count").unwrap();
    Some((amt, cnt))
}

#[tokio::test]
async fn update_holder_inserts_new_row() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 500, 100)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((500, 1))
    );
}

#[tokio::test]
async fn update_holder_increments_existing() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 200, 101)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((300, 2))
    );
}

#[tokio::test]
async fn negative_delta_cleans_up_zero_balance() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", -100, 101)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        None,
        "row should be deleted when balance hits zero"
    );
}

#[tokio::test]
async fn update_holders_batch_groups_per_app_address_pair() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holders_batch(
        vec![
            ("t/x/y".to_string(), "bc1qaaa".to_string(), 100, 100),
            ("t/x/y".to_string(), "bc1qaaa".to_string(), 50, 101),
            ("t/x/y".to_string(), "bc1qbbb".to_string(), 200, 100),
        ],
        "mainnet",
    )
    .await
    .unwrap();

    // grouped: aaa gets 150 in one UPSERT (so charm_count = 1, not 2)
    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((150, 1))
    );
    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qbbb", "mainnet").await,
        Some((200, 1))
    );
}

/// Audit finding N1 — STILL OPEN: re-processing the same block doubles
/// balances because the UPSERT increments unconditionally. The fix
/// requires idempotency keys or DB transactions (planned for T3.4).
#[tokio::test]
async fn reprocessing_doubles_balance_known_bug() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();
    // simulating a crash + restart that re-runs the same block
    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((200, 2)),
        "current behaviour: balance doubles on reprocess (audit N1)"
    );
}

/// Regression test for audit finding N2 (FIXED): mainnet and testnet4
/// balances for the same (app_id, address) live in separate rows.
#[tokio::test]
async fn cross_network_balances_are_isolated() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", "testnet4", 50, 100)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((100, 1))
    );
    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "testnet4").await,
        Some((50, 1))
    );
}
