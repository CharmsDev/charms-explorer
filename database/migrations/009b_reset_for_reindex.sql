-- Script: Reset database for reindex (PRESERVES: transactions, spells)
-- Run this BEFORE starting the indexer to reprocess all confirmed blocks
-- Usage: psql -U charms_user -h localhost -p 8003 -d charms_indexer -f 009b_reset_for_reindex.sql
--
-- PRESERVES: transactions (downloaded data), spells (parsed spells)
-- CLEARS: charms, assets, stats_holders (calculated data)
-- INITIALIZES: block_status from existing transactions

BEGIN;

-- 1. Clear calculated tables (NOT transactions, NOT spells)
TRUNCATE TABLE stats_holders;

DELETE FROM charms;

DELETE FROM assets;

-- 2. Reset bookmark to start from beginning
UPDATE bookmark SET height = 0 WHERE height > 0;

-- 3. Reset summary stats
UPDATE summary
SET
    total_charms = 0,
    nft_count = 0,
    token_count = 0,
    dapp_count = 0,
    other_count = 0,
    last_processed_block = 0;

-- 4. Clear and repopulate block_status from existing transactions
TRUNCATE TABLE block_status;

-- 5. Populate block_status from transactions table
-- All existing blocks are considered downloaded and confirmed (historical data)
INSERT INTO
    block_status (
        block_height,
        network,
        blockchain,
        downloaded,
        processed,
        confirmed,
        tx_count,
        downloaded_at,
        created_at,
        updated_at
    )
SELECT
    block_height,
    network,
    blockchain,
    true as downloaded, -- Already downloaded
    false as processed, -- Needs reprocessing
    true as confirmed, -- Historical = confirmed
    COUNT(*) as tx_count,
    MIN(updated_at) as downloaded_at,
    NOW() as created_at,
    NOW() as updated_at
FROM transactions
GROUP BY
    block_height,
    network,
    blockchain
ON CONFLICT (
    block_height,
    network,
    blockchain
) DO
UPDATE
SET
    downloaded = true,
    processed = false,
    confirmed = true,
    tx_count = EXCLUDED.tx_count,
    updated_at = NOW();

COMMIT;

-- Summary of what was done:
SELECT 'block_status populated' as action, COUNT(*) as count
FROM block_status
UNION ALL
SELECT 'transactions preserved', COUNT(*)
FROM transactions
UNION ALL
SELECT 'spells preserved', COUNT(*)
FROM spells
UNION ALL
SELECT 'charms cleared', 0
UNION ALL
SELECT 'assets cleared', 0
UNION ALL
SELECT 'stats_holders cleared', 0;

-- After running this script, start the indexer/reindexer normally.
-- It will process all blocks that have downloaded=true AND processed=false