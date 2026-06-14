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

/// Regression test for audit finding N1 (FIXED): re-processing the same
/// block after a crash must not double the balance. The repository now
/// gates the DO UPDATE on `last_updated_block < {block}`.
#[tokio::test]
async fn reprocessing_same_block_is_idempotent() {
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
        Some((100, 1)),
        "must not double-count on reprocess (audit N1)"
    );
}

/// Higher-block updates still go through — the gate is `< block`, not `<=`.
#[tokio::test]
async fn later_block_does_apply_after_idempotent_skip() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap();
    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 100, 100)
        .await
        .unwrap(); // skipped
    repo.update_holder_stats("t/x/y", "bc1qaaa", "mainnet", 50, 101)
        .await
        .unwrap(); // applies

    assert_eq!(
        row(&db.conn, "t/x/y", "bc1qaaa", "mainnet").await,
        Some((150, 2))
    );
}

/// Regression for anomaly A1: within a single block, an address can both
/// gain and lose balance (the classic "spend with change" pattern). The
/// repo's idempotency gate (`last_updated_block < block`) requires the
/// caller to MERGE the additive and subtractive deltas to a single net
/// value before invoking `update_holders_batch`. This test enforces that
/// contract by replaying the merged delta directly.
#[tokio::test]
async fn within_block_merged_delta_lands_correctly() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holders_batch(
        vec![("n/x/y".to_string(), "addrA".to_string(), 100, 100)],
        "mainnet",
    )
    .await
    .unwrap();
    assert_eq!(
        row(&db.conn, "n/x/y", "addrA", "mainnet").await,
        Some((100, 1))
    );

    // Block 101: A spent 100, received 90 change. Block processor merges
    // (+90) and (-100) into a single (-10) net delta.
    repo.update_holders_batch(
        vec![("n/x/y".to_string(), "addrA".to_string(), -10, 101)],
        "mainnet",
    )
    .await
    .unwrap();
    // total_amount correctly reflects the net change. charm_count is
    // a coarse heuristic derived from the sign of the merged delta and is
    // not strictly equal to "unspent UTXOs"; it is tracked separately in
    // _rjj/log/test-mainnet-from-genesis.md as anomaly A1b (open).
    let after = row(&db.conn, "n/x/y", "addrA", "mainnet").await.unwrap();
    assert_eq!(after.0, 90, "total_amount must reflect net delta");
}

/// Companion to the merged-delta test: when the merged delta zeroes out
/// the balance (sender spends entire holdings, no change), the row is
/// removed by `cleanup_zero_holders` instead of lingering as a ghost row.
#[tokio::test]
async fn within_block_full_spend_drops_to_zero() {
    let db = TestDb::new().await;
    let repo = StatsHoldersRepository::new(db.conn.clone());

    repo.update_holders_batch(
        vec![("n/x/y".to_string(), "addrA".to_string(), 100, 100)],
        "mainnet",
    )
    .await
    .unwrap();

    repo.update_holders_batch(
        vec![("n/x/y".to_string(), "addrA".to_string(), -100, 101)],
        "mainnet",
    )
    .await
    .unwrap();

    assert_eq!(row(&db.conn, "n/x/y", "addrA", "mainnet").await, None);
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
