// mempool.space broadcast service — primary broadcast provider.
// Uses Core v30+ nodes with relaxed datacarriersize, ensuring large OP_RETURN txs propagate.

/// Broadcast a raw transaction via mempool.space. Returns txid on success.
pub async fn broadcast(
    http_client: &reqwest::Client,
    raw_tx_hex: &str,
    network: &str,
) -> Result<String, String> {
    let url = if network == "testnet4" {
        "https://mempool.space/testnet4/api/tx"
    } else {
        "https://mempool.space/api/tx"
    };
    let resp = http_client
        .post(url)
        .header("Content-Type", "text/plain")
        .body(raw_tx_hex.to_string())
        .send()
        .await
        .map_err(|e| format!("mempool.space request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(body.trim().to_string())
    } else {
        Err(format!("mempool.space rejected: {} - {}", status, body))
    }
}
