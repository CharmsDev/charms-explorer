//! Integration tests for `StatsHoldersRepository`.
//!
//! Includes regression markers for audit findings N1 (non-idempotent on
//! re-processed blocks) and N2 (no network column → cross-network leak).

mod common;

use charms_indexer::infrastructure::persistence::repositories::StatsHoldersRepository;
use common::TestDb;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

async fn row(
    conn: &sea_orm::DatabaseConnection,
    app_id: &str,
    address: &str,
) -> Option<(i64, i32)> {
    let sql = format!(
        "SELECT total_amount, charm_count FROM stats_holders \
         WHERE app_id = '{}' AND address = '{}'",
        app_id, address
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

    repo.update_holder_stats("t/x/y", "bc1qaaa", 500, 100)
        .await
        .unwrap();

    assert_eq!(row(&db.conn, "t/x/y", "bc1qaaa").await, Some((500, 1)));
}

#[tokio::test]
async fn update_holder_increments_existing() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", 200, 101)
        .await
        .unwrap();

    assert_eq!(row(&db.conn, "t/x/y", "bc1qaaa").await, Some((300, 2)));
}

#[tokio::test]
async fn negative_delta_cleans_up_zero_balance() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", -100, 101)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa").await,
        None,
        "row should be deleted when balance hits zero"
    );
}

#[tokio::test]
async fn update_holders_batch_groups_per_app_address_pair() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holders_batch(vec![
        ("t/x/y".to_string(), "bc1qaaa".to_string(), 100, 100),
        ("t/x/y".to_string(), "bc1qaaa".to_string(), 50, 101),
        ("t/x/y".to_string(), "bc1qbbb".to_string(), 200, 100),
    ])
    .await
    .unwrap();

    // grouped: aaa gets 150 in one UPSERT (so charm_count = 1, not 2)
    assert_eq!(row(&db.conn, "t/x/y", "bc1qaaa").await, Some((150, 1)));
    assert_eq!(row(&db.conn, "t/x/y", "bc1qbbb").await, Some((200, 1)));
}

/// Audit finding N1: re-processing the same block doubles balances because
/// the UPSERT increments unconditionally — there is no idempotency key.
#[tokio::test]
async fn reprocessing_doubles_balance_known_bug() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", 100, 100)
        .await
        .unwrap();
    // simulating a crash + restart that re-runs the same block
    repo.update_holder_stats("t/x/y", "bc1qaaa", 100, 100)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa").await,
        Some((200, 2)),
        "current behaviour: balance doubles on reprocess (audit N1)"
    );
}

/// Audit finding N2: PK is `(app_id, address)` with no `network` column.
/// Two networks with the same (app_id, address) collide into one row.
#[tokio::test]
async fn cross_network_balances_collide_known_bug() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    // "mainnet" delta
    repo.update_holder_stats("t/x/y", "bc1qaaa", 100, 100)
        .await
        .unwrap();
    // "testnet4" delta (no way to express network at this API level)
    repo.update_holder_stats("t/x/y", "bc1qaaa", 50, 100)
        .await
        .unwrap();

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa").await,
        Some((150, 2)),
        "current behaviour: balances of different networks merge (audit N2)"
    );
}
