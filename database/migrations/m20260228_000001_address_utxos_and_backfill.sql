-- Migration: Add address_utxos table + backfill monitored_addresses from charms
-- Date: 2026-02-28
-- Applies to: production (fly-db)

-- 1. Create address_utxos table (BTC UTXO tracking for monitored addresses)
CREATE TABLE IF NOT EXISTS address_utxos (
    txid          VARCHAR(64)  NOT NULL,
    vout          INTEGER      NOT NULL,
    network       VARCHAR(10)  NOT NULL DEFAULT 'mainnet',
    address       VARCHAR(62)  NOT NULL,
    value         BIGINT       NOT NULL,
    script_pubkey VARCHAR(140) NOT NULL DEFAULT '',
    block_height  INTEGER,
    PRIMARY KEY (txid, vout, network)
);

CREATE INDEX IF NOT EXISTS idx_address_utxos_address ON address_utxos (address, network);
CREATE INDEX IF NOT EXISTS idx_address_utxos_block ON address_utxos (block_height, network);

-- 2. Backfill monitored_addresses from existing charms (register all charm holders)
INSERT INTO monitored_addresses (address, network, source, created_at)
SELECT DISTINCT address, network, 'backfill', NOW()
FROM charms
WHERE address IS NOT NULL AND address != '' AND spent = false
ON CONFLICT (address, network) DO NOTHING;

-- 3. Register migration
INSERT INTO seaql_migrations (version) VALUES ('m20260228_000001_address_utxos_and_backfill')
ON CONFLICT DO NOTHING;
