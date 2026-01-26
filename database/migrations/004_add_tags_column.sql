-- Migration: Add tags column for DEX and other categorization
-- Date: 2026-01-26
-- Description: Adds tags column to charms and transactions tables for categorizing
--              DEX operations (charms-cast) and other future use cases.

-- Add tags column to charms table
ALTER TABLE charms ADD COLUMN IF NOT EXISTS tags VARCHAR(255) DEFAULT NULL;

-- Add tags column to transactions table  
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS tags VARCHAR(255) DEFAULT NULL;

-- Create indexes for efficient tag-based queries
CREATE INDEX IF NOT EXISTS idx_charms_tags ON charms(tags);
CREATE INDEX IF NOT EXISTS idx_transactions_tags ON transactions(tags);

-- Create partial index for non-null tags (more efficient for filtering DEX transactions)
CREATE INDEX IF NOT EXISTS idx_charms_tags_notnull ON charms(tags) WHERE tags IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_transactions_tags_notnull ON transactions(tags) WHERE tags IS NOT NULL;

-- Comment on columns
COMMENT ON COLUMN charms.tags IS 'Comma-separated tags for categorization (e.g., charms-cast,create-ask)';
COMMENT ON COLUMN transactions.tags IS 'Comma-separated tags for categorization (e.g., charms-cast,fulfill-bid)';
