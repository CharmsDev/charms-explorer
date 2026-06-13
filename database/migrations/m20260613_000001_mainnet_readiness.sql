-- Migration: m20260613_000001_mainnet_readiness
-- Purpose: prepare the indexer for a fresh-from-genesis mainnet run with
--          reorg detection, Maestro↔node handoff integrity, and hot-path
--          indexes. Groups M1–M4 from _rjj/context/indexer-schema.md.
-- Safe: only adds columns/tables/indexes and backfills NULLs before
--       tightening constraints. No destructive operations.

-- ============================================================
-- M1 — Reorg readiness
-- ============================================================

ALTER TABLE block_status
    ADD COLUMN IF NOT EXISTS previous_block_hash TEXT;

UPDATE block_status
    SET block_hash = 'unknown-pre-migration'
    WHERE block_hash IS NULL;

ALTER TABLE block_status
    ALTER COLUMN block_hash SET NOT NULL;

CREATE TABLE IF NOT EXISTS reorg_events (
    id            SERIAL      PRIMARY KEY,
    network       TEXT        NOT NULL,
    from_height   INTEGER     NOT NULL,
    depth         INTEGER     NOT NULL,
    detected_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    recovered_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_reorg_events_network
    ON reorg_events (network, detected_at DESC);

-- ============================================================
-- M2 — Maestro↔node handoff integrity
-- ============================================================

ALTER TABLE monitored_addresses
    ADD COLUMN IF NOT EXISTS seed_block_hash TEXT;

ALTER TABLE address_utxos
    ADD COLUMN IF NOT EXISTS source TEXT
    CHECK (source IS NULL OR source IN ('maestro', 'node', 'backfill'));

CREATE TABLE IF NOT EXISTS sync_discrepancies (
    id           SERIAL      PRIMARY KEY,
    address      TEXT        NOT NULL,
    network      TEXT        NOT NULL,
    txid         TEXT        NOT NULL,
    vout         INTEGER     NOT NULL,
    source_a     TEXT        NOT NULL,
    source_b     TEXT        NOT NULL,
    value_a      BIGINT,
    value_b      BIGINT,
    detected_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sync_discrepancies_address
    ON sync_discrepancies (address, network, detected_at DESC);

-- ============================================================
-- M3 — Hot-path indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_charms_address_net_spent
    ON charms (address, network, spent);

CREATE INDEX IF NOT EXISTS idx_charms_appid_net_spent
    ON charms (app_id, network, spent);

CREATE INDEX IF NOT EXISTS idx_charms_block_height
    ON charms (block_height, network)
    WHERE block_height IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_transactions_net_block
    ON transactions (network, block_height DESC);

CREATE INDEX IF NOT EXISTS idx_dex_orders_asset_status
    ON dex_orders (asset_app_id, network, status);

CREATE INDEX IF NOT EXISTS idx_dex_orders_maker
    ON dex_orders (maker, network);

-- ============================================================
-- Register migration
-- ============================================================

INSERT INTO seaql_migrations (version)
VALUES ('m20260613_000001_mainnet_readiness')
ON CONFLICT (version) DO NOTHING;
