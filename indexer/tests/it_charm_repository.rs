//! Integration tests for `CharmRepository` against an ephemeral Postgres.

mod common;

use charms_indexer::infrastructure::persistence::repositories::CharmRepository;
use common::TestDb;
use serde_json::json;

type Row = (
    String,
    i32,
    u64,
    serde_json::Value,
    String,
    String,
    String,
    Option<String>,
    String,
    i64,
    Option<String>,
);

fn charm_row(
    txid: &str,
    vout: i32,
    network: &str,
    app_id: &str,
    amount: i64,
    tags: Option<&str>,
) -> Row {
    (
        txid.to_string(),
        vout,
        100,
        json!({"amount": amount}),
        "token".to_string(),
        "Bitcoin".to_string(),
        network.to_string(),
        Some("bc1qxxx".to_string()),
        app_id.to_string(),
        amount,
        tags.map(String::from),
    )
}

#[tokio::test]
async fn save_batch_inserts_new_charms_and_reports_them() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    let batch = vec![
        charm_row("aa", 0, "mainnet", "t/x/y", 100, None),
        charm_row("aa", 1, "mainnet", "t/x/y", 50, None),
    ];

    let inserted = repo.save_batch(batch).await.expect("save");
    assert_eq!(inserted.len(), 2);
    assert!(inserted.contains(&("aa".to_string(), 0)));
    assert!(inserted.contains(&("aa".to_string(), 1)));
}

#[tokio::test]
async fn save_batch_is_idempotent_on_conflict() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    let row = charm_row("bb", 0, "mainnet", "t/x/y", 100, None);
    let first = repo.save_batch(vec![row.clone()]).await.expect("first");
    assert_eq!(first.len(), 1);

    // second call with the same (txid, vout) returns nothing (ON CONFLICT DO NOTHING)
    let second = repo.save_batch(vec![row]).await.expect("second");
    assert!(second.is_empty(), "duplicate insert should not re-report");
}

#[tokio::test]
async fn mark_charms_as_spent_batch_flips_spent_flag() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    repo.save_batch(vec![
        charm_row("cc", 0, "mainnet", "t/x/y", 10, None),
        charm_row("cc", 1, "mainnet", "t/x/y", 20, None),
    ])
    .await
    .expect("save");

    repo.mark_charms_as_spent_batch(vec![("cc".to_string(), 0)])
        .await
        .expect("mark spent");

    let still_unspent = repo
        .get_unspent_charms_by_txid_vout(vec![
            ("cc".to_string(), 0),
            ("cc".to_string(), 1),
        ])
        .await
        .expect("query");
    assert_eq!(still_unspent.len(), 1);
    assert_eq!(still_unspent[0].1, 1, "vout=1 should remain unspent");
}

#[tokio::test]
async fn has_beam_out_input_txid_detects_beam_out_tag() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    repo.save_batch(vec![
        charm_row("beam_out_txid", 0, "mainnet", "t/x/y", 100, Some("beam-out,bro")),
        charm_row("plain_txid", 0, "mainnet", "t/x/y", 100, Some("bro-mint")),
    ])
    .await
    .expect("save");

    assert!(repo
        .has_beam_out_input_txid(&["beam_out_txid".to_string()])
        .await
        .unwrap());
    assert!(!repo
        .has_beam_out_input_txid(&["plain_txid".to_string()])
        .await
        .unwrap());
    assert!(!repo.has_beam_out_input_txid(&[]).await.unwrap());
}

/// Audit finding N5: `mark_charms_as_spent_batch` does not filter by network,
/// so a (txid, vout) collision across networks would corrupt the wrong row.
/// This test documents the current behaviour — both rows are marked spent.
#[tokio::test]
async fn mark_spent_batch_does_not_isolate_networks_known_bug() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    repo.save_batch(vec![
        charm_row("dd", 0, "mainnet", "t/x/y", 10, None),
        charm_row("dd", 0, "testnet4", "t/x/y", 10, None),
    ])
    .await
    .expect("save");

    repo.mark_charms_as_spent_batch(vec![("dd".to_string(), 0)])
        .await
        .expect("mark");

    let remaining = repo
        .get_unspent_charms_by_txid_vout(vec![("dd".to_string(), 0)])
        .await
        .expect("query");
    assert!(
        remaining.is_empty(),
        "current behaviour marks BOTH networks spent (audit N5)"
    );
}

#[tokio::test]
async fn get_distinct_block_heights_filters_by_network_and_ignores_null() {
    let db = TestDb::new().await;
    let repo = CharmRepository::new(db.conn.clone());

    let mut row1 = charm_row("ee", 0, "mainnet", "t/x/y", 10, None);
    row1.2 = 100;
    let mut row2 = charm_row("ee", 1, "mainnet", "t/x/y", 10, None);
    row2.2 = 200;
    let mut row_other_net = charm_row("ee", 2, "testnet4", "t/x/y", 10, None);
    row_other_net.2 = 300;

    repo.save_batch(vec![row1, row2, row_other_net])
        .await
        .expect("save");

    let mainnet_heights = repo
        .get_distinct_block_heights("mainnet")
        .await
        .expect("query");
    assert_eq!(mainnet_heights, vec![100, 200]);

    let testnet_heights = repo
        .get_distinct_block_heights("testnet4")
        .await
        .expect("query");
    assert_eq!(testnet_heights, vec![300]);
}
