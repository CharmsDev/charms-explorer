-- Migration: m20260622_000001_assets_unique_app_id_network
-- Purpose: the assets UNIQUE constraint was on (app_id) alone, which made the
-- same token app_id collide across networks. Mainnet's FIRE token row (mint at
-- block 953,330) blocked the equivalent testnet4 row from being inserted at
-- block 141,026, so the testnet4 charm was indexed but missing from the assets
-- table and the explorer showed no metadata for it.
--
-- New constraint includes the network so the same app_id can have one row per
-- network (mainnet, testnet, testnet4). Existing data is already unique on
-- (app_id, network) by virtue of the stricter old constraint, so the swap is
-- collision-free.

ALTER TABLE assets DROP CONSTRAINT IF EXISTS assets_app_id_key;
ALTER TABLE assets ADD CONSTRAINT assets_app_id_network_key UNIQUE (app_id, network);

INSERT INTO seaql_migrations (version)
VALUES ('m20260622_000001_assets_unique_app_id_network')
ON CONFLICT (version) DO NOTHING;
