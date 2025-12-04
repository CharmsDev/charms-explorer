-- Migration: Add Mempool Support
-- Date: 2025-02-04
-- Description: Enable tracking of pending transactions in mempool by making block_height nullable
--              and adding timestamp for mempool detection

-- ============================================================================
-- CHARMS TABLE MODIFICATIONS
-- ============================================================================

-- Make block_height nullable to support mempool charms (not yet in a block)
ALTER TABLE charms ALTER COLUMN block_height DROP NOT NULL;

-- Add timestamp for when charm was first detected in mempool
ALTER TABLE charms
ADD COLUMN IF NOT EXISTS mempool_detected_at TIMESTAMPTZ;

-- Add partial index for efficient mempool queries (only indexes rows where block_height IS NULL)
CREATE INDEX IF NOT EXISTS idx_charms_mempool ON charms (network, blockchain)
WHERE
    block_height IS NULL;

-- Add index for ordering mempool charms by detection time
CREATE INDEX IF NOT EXISTS idx_charms_mempool_time ON charms (mempool_detected_at DESC)
WHERE
    block_height IS NULL;

-- Add index for finding charms by txid (for reconciliation when block is mined)
CREATE INDEX IF NOT EXISTS idx_charms_txid ON charms (txid);

-- ============================================================================
-- TRANSACTIONS TABLE MODIFICATIONS
-- ============================================================================

-- Make block_height nullable to support mempool transactions
ALTER TABLE transactions ALTER COLUMN block_height DROP NOT NULL;

-- Add timestamp for when transaction was first detected in mempool
ALTER TABLE transactions
ADD COLUMN IF NOT EXISTS mempool_detected_at TIMESTAMPTZ;

-- Add partial index for efficient mempool queries
CREATE INDEX IF NOT EXISTS idx_transactions_mempool ON transactions (network, blockchain)
WHERE
    block_height IS NULL;

-- Add index for ordering mempool transactions by detection time
CREATE INDEX IF NOT EXISTS idx_transactions_mempool_time ON transactions (mempool_detected_at DESC)
WHERE
    block_height IS NULL;

-- ============================================================================
-- COMMENTS FOR DOCUMENTATION
-- ============================================================================

COMMENT ON COLUMN charms.block_height IS 'Block height where charm was confirmed. NULL if still in mempool (pending).';

COMMENT ON COLUMN charms.mempool_detected_at IS 'Timestamp when charm was first detected in mempool. NULL if charm was never in mempool (directly indexed from block).';

COMMENT ON COLUMN transactions.block_height IS 'Block height where transaction was confirmed. NULL if still in mempool (pending).';

COMMENT ON COLUMN transactions.mempool_detected_at IS 'Timestamp when transaction was first detected in mempool. NULL if transaction was never in mempool (directly indexed from block).';

-- ============================================================================
-- MARK MIGRATION AS APPLIED
-- ============================================================================

INSERT INTO
    seaql_migrations (version)
VALUES (
        'm20250204_000001_add_mempool_support'
    )
ON CONFLICT (version) DO NOTHING;