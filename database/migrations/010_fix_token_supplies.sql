-- Fix token supplies by recalculating from unspent charms
-- This is a one-time fix for the reindexer bug that didn't decrement supply on spent charms
-- Run this script once to correct the data

-- Step 1: Show current state (for verification)
SELECT 
    a.app_id,
    a.name,
    a.total_supply::text as current_supply,
    (a.total_supply / 100000000)::bigint as current_M
FROM assets a
WHERE a.asset_type = 'token'
AND a.name = 'Bro'
ORDER BY a.total_supply DESC;

-- Step 2: Calculate correct supply from unspent charms
-- This creates a temporary view of what the supplies SHOULD be
WITH correct_supplies AS (
    SELECT 
        -- Extract base app_id (without :N suffix) for grouping
        CASE 
            WHEN app_id ~ ':[0-9]+$' THEN regexp_replace(app_id, ':[0-9]+$', '')
            ELSE app_id
        END as base_app_id,
        SUM(amount) as correct_supply
    FROM charms
    WHERE spent = false
    AND app_id LIKE 't/%'
    GROUP BY 
        CASE 
            WHEN app_id ~ ':[0-9]+$' THEN regexp_replace(app_id, ':[0-9]+$', '')
            ELSE app_id
        END
)
SELECT 
    cs.base_app_id,
    cs.correct_supply::text,
    (cs.correct_supply / 100000000)::bigint as correct_M
FROM correct_supplies cs
ORDER BY cs.correct_supply DESC
LIMIT 10;

-- Step 3: Update all token assets with correct supply from unspent charms
-- This is the actual fix
UPDATE assets a
SET total_supply = COALESCE(cs.correct_supply, 0),
    updated_at = NOW()
FROM (
    SELECT 
        CASE 
            WHEN app_id ~ ':[0-9]+$' THEN regexp_replace(app_id, ':[0-9]+$', '')
            ELSE app_id
        END as base_app_id,
        SUM(amount) as correct_supply
    FROM charms
    WHERE spent = false
    AND app_id LIKE 't/%'
    GROUP BY 
        CASE 
            WHEN app_id ~ ':[0-9]+$' THEN regexp_replace(app_id, ':[0-9]+$', '')
            ELSE app_id
        END
) cs
WHERE a.asset_type = 'token'
AND (
    -- Match assets where app_id starts with the base_app_id
    a.app_id LIKE cs.base_app_id || '%'
);

-- Step 4: Verify the fix
SELECT 
    a.app_id,
    a.name,
    a.total_supply::text as new_supply,
    (a.total_supply / 100000000)::bigint as new_M
FROM assets a
WHERE a.asset_type = 'token'
AND a.name = 'Bro'
ORDER BY a.total_supply DESC;
