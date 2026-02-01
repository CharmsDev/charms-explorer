-- Script: Fix total_supply in assets table after reindex
-- Run this AFTER reindexing to correct total_supply values
-- Usage: psql -U charms_user -h localhost -p 8003 -d charms_indexer -f 009c_fix_total_supply.sql
--
-- PROBLEM: save_batch uses data.supply (which is empty) instead of charm.amount
-- SOLUTION: Recalculate total_supply from sum of unspent charm amounts

BEGIN;

-- Update total_supply for tokens based on sum of UNSPENT charm amounts
-- Only tokens (t/) need fixing - NFTs have supply = 0
UPDATE assets a
SET total_supply = COALESCE(c.total_amount, 0)
FROM (
    SELECT 
        app_id,
        SUM(amount) as total_amount
    FROM charms 
    WHERE spent = false
    GROUP BY app_id
) c
WHERE a.app_id = c.app_id
AND a.asset_type = 'token';

-- Verify the fix
SELECT 
    'Token supplies updated' as action,
    COUNT(*) as tokens_fixed
FROM assets 
WHERE asset_type = 'token';

COMMIT;

-- Show sample results
SELECT 
    a.app_id,
    a.name,
    a.total_supply as new_supply,
    c.unspent_amount
FROM assets a
LEFT JOIN (
    SELECT app_id, SUM(amount) as unspent_amount
    FROM charms WHERE spent = false
    GROUP BY app_id
) c ON a.app_id = c.app_id
WHERE a.asset_type = 'token'
ORDER BY a.total_supply DESC
LIMIT 10;
