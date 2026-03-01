-- Migration: m20260301_000001_transactions_mempool_columns
-- Purpose: Add mempool tracking columns to transactions table
-- Safe: only adds nullable columns, no data loss
-- Required before enabling mempool_detected_at / tags in the API entity
-- Author: RJJ

-- 1. Add mempool detection timestamp (NULL = confirmed tx, non-NULL = mempool-detected)
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS mempool_detected_at TIMESTAMP;

-- 2. Add tags column (comma-separated: bro, charms-cast, beaming, etc.)
ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS tags VARCHAR(255);

-- 3. Register migration
INSERT INTO seaql_migrations (version)
VALUES ('m20260301_000001_transactions_mempool_columns')
ON CONFLICT (version) DO NOTHING;
