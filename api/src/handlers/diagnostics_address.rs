//! Diagnostics endpoint for a single address.
//!
//! `GET /internal/diagnostics/address/:network/:addr` — returns the full
//! reconciliation state of a monitored address: seed cursor, indexer view,
//! UTXO/tx counts, balance breakdown. Used during the mainnet-from-genesis
//! test to manually verify that the indexer + Maestro handoff is hermetic.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement};
use serde_json::{json, Value};

use crate::handlers::AppState;

pub async fn diagnostics_address(
    State(app_state): State<AppState>,
    Path((network, address)): Path<(String, String)>,
) -> impl IntoResponse {
    let conn = app_state.repositories.charm.get_connection();

    let monitored = fetch_monitored(&conn, &address, &network).await;
    let indexer_view = match &monitored {
        Some(m) => indexer_block_hash_at(&conn, &network, m.seed_height).await,
        None => None,
    };
    let cursor_valid = match (&monitored, &indexer_view) {
        (Some(m), Some(stored)) => match (&m.seed_block_hash, stored) {
            (Some(expected), stored_hash) => Some(expected == stored_hash),
            _ => None,
        },
        _ => None,
    };

    let utxos = fetch_utxos(&conn, &address, &network).await;
    let tx_count = fetch_tx_count(&conn, &address, &network).await;
    let mempool_spent = fetch_mempool_spent(&conn, &address, &network).await;

    let confirmed: i64 = utxos
        .iter()
        .filter(|u| u.block_height.unwrap_or(0) > 0)
        .map(|u| u.value)
        .sum();
    let unconfirmed: i64 = utxos
        .iter()
        .filter(|u| u.block_height.unwrap_or(0) == 0)
        .map(|u| u.value)
        .sum();

    let body = json!({
        "address": address,
        "network": network,
        "monitored": monitored.as_ref().map(|m| json!({
            "source": m.source,
            "seeded_at": m.seeded_at,
            "seed_height": m.seed_height,
            "seed_block_hash": m.seed_block_hash,
        })),
        "handoff": {
            "indexer_block_hash_at_seed_height": indexer_view,
            "cursor_valid": cursor_valid,
            "interpretation": describe_cursor(&cursor_valid, &monitored),
        },
        "balance_sats": {
            "confirmed": confirmed,
            "unconfirmed": unconfirmed,
            "total": confirmed + unconfirmed,
        },
        "counts": {
            "utxos": utxos.len(),
            "address_transactions": tx_count,
            "mempool_spent": mempool_spent,
        },
        "utxos_sample": utxos.into_iter().take(10).map(|u| json!({
            "txid": u.txid,
            "vout": u.vout,
            "value": u.value,
            "block_height": u.block_height,
            "source": u.source,
        })).collect::<Vec<_>>(),
    });

    Json(body)
}

#[derive(FromQueryResult)]
struct MonitoredRow {
    source: String,
    seeded_at: Option<chrono::DateTime<chrono::Utc>>,
    seed_height: Option<i32>,
    seed_block_hash: Option<String>,
}

async fn fetch_monitored(
    conn: &sea_orm::DatabaseConnection,
    address: &str,
    network: &str,
) -> Option<MonitoredRow> {
    MonitoredRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT source, seeded_at, seed_height, seed_block_hash \
         FROM monitored_addresses WHERE address = $1 AND network = $2",
        [address.into(), network.into()],
    ))
    .one(conn)
    .await
    .ok()
    .flatten()
}

async fn indexer_block_hash_at(
    conn: &sea_orm::DatabaseConnection,
    network: &str,
    height: Option<i32>,
) -> Option<String> {
    let height = height?;
    #[derive(FromQueryResult)]
    struct Row {
        block_hash: Option<String>,
    }
    Row::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT block_hash FROM block_status \
         WHERE block_height = $1 AND network = $2 LIMIT 1",
        [height.into(), network.into()],
    ))
    .one(conn)
    .await
    .ok()
    .flatten()
    .and_then(|r| r.block_hash)
}

#[derive(FromQueryResult)]
struct UtxoRow {
    txid: String,
    vout: i32,
    value: i64,
    block_height: Option<i32>,
    source: Option<String>,
}

async fn fetch_utxos(
    conn: &sea_orm::DatabaseConnection,
    address: &str,
    network: &str,
) -> Vec<UtxoRow> {
    UtxoRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT txid, vout, value, block_height, source FROM address_utxos \
         WHERE address = $1 AND network = $2 \
         ORDER BY block_height DESC NULLS FIRST",
        [address.into(), network.into()],
    ))
    .all(conn)
    .await
    .unwrap_or_default()
}

async fn fetch_tx_count(
    conn: &sea_orm::DatabaseConnection,
    address: &str,
    network: &str,
) -> i64 {
    #[derive(FromQueryResult)]
    struct Row {
        n: i64,
    }
    Row::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT COUNT(*)::bigint AS n FROM address_transactions \
         WHERE address = $1 AND network = $2",
        [address.into(), network.into()],
    ))
    .one(conn)
    .await
    .ok()
    .flatten()
    .map(|r| r.n)
    .unwrap_or(0)
}

async fn fetch_mempool_spent(
    conn: &sea_orm::DatabaseConnection,
    address: &str,
    network: &str,
) -> i64 {
    #[derive(FromQueryResult)]
    struct Row {
        n: i64,
    }
    let sql = "SELECT COUNT(*)::bigint AS n FROM mempool_spends ms \
               INNER JOIN address_utxos au \
                 ON ms.spent_txid = au.txid AND ms.spent_vout = au.vout AND ms.network = au.network \
               WHERE au.address = $1 AND au.network = $2";
    Row::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [address.into(), network.into()],
    ))
    .one(conn)
    .await
    .ok()
    .flatten()
    .map(|r| r.n)
    .unwrap_or(0)
}

fn describe_cursor(
    cursor_valid: &Option<bool>,
    monitored: &Option<MonitoredRow>,
) -> Value {
    match (cursor_valid, monitored) {
        (Some(true), _) => json!("seed cursor matches indexer view; handoff hermetic"),
        (Some(false), _) => json!(
            "seed_block_hash DIFFERS from indexer block_hash at seed_height — reorg between seed and indexer; will be re-seeded on next API request"
        ),
        (None, Some(m)) if m.seed_block_hash.is_none() => {
            json!("legacy seed without hash; cannot validate cursor")
        }
        (None, Some(_)) => json!("indexer has not reached seed_height yet"),
        (None, None) => json!("address not monitored"),
    }
}
