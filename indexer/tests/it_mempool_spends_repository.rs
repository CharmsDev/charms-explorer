//! Integration tests for `MempoolSpendsRepository` — focused on the
//! RBF / double-spend resolution policy (last-spender-wins).

mod common;

use charms_indexer::infrastructure::persistence::repositories::MempoolSpendsRepository;
use common::TestDb;
use sea_orm::{DbBackend, FromQueryResult, Statement};

#[derive(FromQueryResult, Debug, PartialEq, Eq)]
struct SpendRow {
    spending_txid: String,
    spent_txid: String,
    spent_vout: i32,
}

async fn fetch_spend(
    conn: &sea_orm::DatabaseConnection,
    spent_txid: &str,
    spent_vout: i32,
    network: &str,
) -> Option<SpendRow> {
    SpendRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT spending_txid, spent_txid, spent_vout FROM mempool_spends \
         WHERE spent_txid = $1 AND spent_vout = $2 AND network = $3",
        [spent_txid.into(), spent_vout.into(), network.into()],
    ))
    .one(conn)
    .await
    .ok()
    .flatten()
}

#[tokio::test]
async fn rbf_replaces_spending_txid() {
    let db = TestDb::new().await;
    let repo = MempoolSpendsRepository::new(db.conn.clone());

    // First spender: tx A grabs UTXO (U, 0)
    repo.record_spends_batch(
        &[("tx_a".to_string(), "utxo_u".to_string(), 0)],
        "mainnet",
    )
    .await
    .unwrap();

    let row = fetch_spend(&db.conn, "utxo_u", 0, "mainnet").await.unwrap();
    assert_eq!(row.spending_txid, "tx_a");

    // RBF: tx B replaces tx A on the same UTXO. Last-spender-wins.
    repo.record_spends_batch(
        &[("tx_b".to_string(), "utxo_u".to_string(), 0)],
        "mainnet",
    )
    .await
    .unwrap();

    let row = fetch_spend(&db.conn, "utxo_u", 0, "mainnet").await.unwrap();
    assert_eq!(
        row.spending_txid, "tx_b",
        "ON CONFLICT must overwrite spending_txid so the API reflects the live mempool spender"
    );
}

#[tokio::test]
async fn cross_network_does_not_collide() {
    let db = TestDb::new().await;
    let repo = MempoolSpendsRepository::new(db.conn.clone());

    repo.record_spends_batch(
        &[("tx_main".to_string(), "utxo_u".to_string(), 0)],
        "mainnet",
    )
    .await
    .unwrap();
    repo.record_spends_batch(
        &[("tx_test".to_string(), "utxo_u".to_string(), 0)],
        "testnet4",
    )
    .await
    .unwrap();

    let main = fetch_spend(&db.conn, "utxo_u", 0, "mainnet").await.unwrap();
    let test = fetch_spend(&db.conn, "utxo_u", 0, "testnet4").await.unwrap();
    assert_eq!(main.spending_txid, "tx_main");
    assert_eq!(test.spending_txid, "tx_test");
}

#[tokio::test]
async fn remove_by_spending_txid_clears_the_row() {
    let db = TestDb::new().await;
    let repo = MempoolSpendsRepository::new(db.conn.clone());

    repo.record_spends_batch(
        &[
            ("tx_a".to_string(), "utxo_u".to_string(), 0),
            ("tx_a".to_string(), "utxo_v".to_string(), 1),
        ],
        "mainnet",
    )
    .await
    .unwrap();

    repo.remove_by_spending_txid("tx_a", "mainnet").await.unwrap();

    assert!(fetch_spend(&db.conn, "utxo_u", 0, "mainnet").await.is_none());
    assert!(fetch_spend(&db.conn, "utxo_v", 1, "mainnet").await.is_none());
}
