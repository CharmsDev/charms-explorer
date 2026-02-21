-- Migration: m20260221_000001_mempool_indexing
-- Purpose: Add mempool indexing support
-- Safe: only adds nullable columns and new table, no data loss
-- Author: RJJ

-- 1. Make address_utxos.block_height nullable (mempool UTXOs have no block yet)
ALTER TABLE address_utxos ALTER COLUMN block_height DROP NOT NULL;

-- 2. Make transactions.block_height nullable (mempool txs have no block yet)
ALTER TABLE transactions ALTER COLUMN block_height DROP NOT NULL;

-- 3. New table: tracks which UTXOs are being spent by mempool transactions
--    This allows computing "available balance" = confirmed UTXOs not spent in mempool
CREATE TABLE IF NOT EXISTS mempool_spends (
    spending_txid VARCHAR(64)  NOT NULL,
    spent_txid    VARCHAR(64)  NOT NULL,
    spent_vout    INTEGER      NOT NULL,
    network       VARCHAR(10)  NOT NULL DEFAULT 'mainnet',
    detected_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (spent_txid, spent_vout, network)
);

CREATE INDEX IF NOT EXISTS idx_mempool_spends_spending ON mempool_spends (spending_txid, network);
CREATE INDEX IF NOT EXISTS idx_mempool_spends_detected ON mempool_spends (detected_at);

-- 4. Register migration
INSERT INTO seaql_migrations (version)
VALUES ('m20260221_000001_mempool_indexing')
ON CONFLICT (version) DO NOTHING;
