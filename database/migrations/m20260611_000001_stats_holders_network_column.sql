-- T3.3 / Audit N2: scope stats_holders by network so mainnet and testnet4
-- balances do not collide on the same (app_id, address).
--
-- IMPORTANT: existing rows are tagged as 'mainnet' by default. If your
-- deployment indexed testnet4 charms into this table, run the indexer's
-- rebuild_stats_holders.sh after applying this migration so the rows are
-- regenerated from the source charms with the correct network value.

ALTER TABLE stats_holders
    ADD COLUMN IF NOT EXISTS network TEXT NOT NULL DEFAULT 'mainnet';

-- Drop the old (app_id, address) UNIQUE and replace with the network-scoped
-- one. The constraint name produced by the original migration is the
-- auto-generated `stats_holders_app_id_address_key`. We also catch the
-- variant produced when the constraint was created without an explicit name
-- by SQL clients.

DO $$
DECLARE
    cname TEXT;
BEGIN
    SELECT conname INTO cname
    FROM pg_constraint
    WHERE conrelid = 'stats_holders'::regclass
      AND contype = 'u'
      AND conkey = (
        SELECT array_agg(attnum ORDER BY attnum)
        FROM pg_attribute
        WHERE attrelid = 'stats_holders'::regclass
          AND attname IN ('app_id', 'address')
      );
    IF cname IS NOT NULL THEN
        EXECUTE format('ALTER TABLE stats_holders DROP CONSTRAINT %I', cname);
    END IF;
END$$;

ALTER TABLE stats_holders
    ADD CONSTRAINT stats_holders_app_id_address_network_key
    UNIQUE (app_id, address, network);
