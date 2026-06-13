//! Prometheus metrics exporter.
//!
//! Spawns an HTTP server on `METRICS_PORT` (default 9000) and exposes
//! `/metrics` for scraping. Disabled if `METRICS_PORT=0`.
//!
//! Conventions:
//! - All metrics carry a `network` label (mainnet, testnet4) where applicable.
//! - Counters end in `_total`.
//! - Durations are histograms in seconds.
//!
//! See `block_processed`, `charm_detected`, `current_height` helpers below
//! for instrumented call-sites.

use std::net::SocketAddr;

use metrics_exporter_prometheus::PrometheusBuilder;

use crate::utils::logging;

/// Initialise the Prometheus exporter. Reads `METRICS_PORT` (default 9000);
/// `METRICS_PORT=0` disables the exporter entirely.
pub fn init() {
    let port = std::env::var("METRICS_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(9000);
    if port == 0 {
        logging::log_info("Metrics exporter disabled (METRICS_PORT=0)");
        return;
    }
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    match PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
    {
        Ok(()) => logging::log_info(&format!("Metrics exporter listening on {addr}/metrics")),
        Err(e) => logging::log_warning(&format!("Failed to start metrics exporter: {e}")),
    }
}

/// Record that a block has been processed.
pub fn block_processed(network: &str, duration_secs: f64) {
    metrics::counter!("indexer_blocks_processed_total", "network" => network.to_string())
        .increment(1);
    metrics::histogram!(
        "indexer_block_processing_duration_seconds",
        "network" => network.to_string()
    )
    .record(duration_secs);
}

/// Record that a charm has been detected for the given asset type.
pub fn charm_detected(network: &str, asset_type: &str) {
    metrics::counter!(
        "indexer_charms_detected_total",
        "network" => network.to_string(),
        "asset_type" => asset_type.to_string()
    )
    .increment(1);
}

/// Update the gauge of the highest processed block height per network.
pub fn current_height(network: &str, height: u64) {
    metrics::gauge!("indexer_current_height", "network" => network.to_string())
        .set(height as f64);
}

/// Update the gauge of the live mempool size as seen by the indexer.
pub fn mempool_size(network: &str, size: usize) {
    metrics::gauge!("indexer_mempool_size", "network" => network.to_string())
        .set(size as f64);
}

/// Record that a supervised task panicked and was restarted. `name` is the
/// supervisor label (e.g. `"mempool/mainnet"`) — same one in the log line.
pub fn supervisor_restart(name: &str) {
    metrics::counter!("indexer_supervisor_restarts_total", "name" => name.to_string())
        .increment(1);
}

/// Record a DEX order detected (mempool or block). `operation` is e.g.
/// `"create_ask"`, `"fulfill_bid"`, `"cancel"`.
pub fn dex_order_detected(network: &str, operation: &str) {
    metrics::counter!(
        "indexer_dex_orders_total",
        "network" => network.to_string(),
        "operation" => operation.to_string()
    )
    .increment(1);
}

/// Record a mempool tx that was reverted by reconcile (RBF / drop).
pub fn mempool_eviction(network: &str) {
    metrics::counter!("indexer_mempool_evictions_total", "network" => network.to_string())
        .increment(1);
}

/// Record a chain reorg detected by the block processor.
pub fn reorg_detected(network: &str, depth: u64) {
    metrics::counter!("indexer_reorgs_total", "network" => network.to_string()).increment(1);
    metrics::histogram!("indexer_reorg_depth", "network" => network.to_string()).record(depth as f64);
}
