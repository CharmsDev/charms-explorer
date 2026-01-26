-- Migration: Add tag statistics columns to summary table
-- Date: 2026-01-26
-- Description: Adds columns for tracking charms by tag (charms-cast, bro, etc.)

-- Add tag count columns to summary table
ALTER TABLE summary ADD COLUMN IF NOT EXISTS charms_cast_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE summary ADD COLUMN IF NOT EXISTS bro_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE summary ADD COLUMN IF NOT EXISTS dex_orders_count BIGINT NOT NULL DEFAULT 0;

-- Comments
COMMENT ON COLUMN summary.charms_cast_count IS 'Number of charms tagged with charms-cast (DEX transactions)';
COMMENT ON COLUMN summary.bro_count IS 'Number of charms tagged with bro ($BRO token transactions)';
COMMENT ON COLUMN summary.dex_orders_count IS 'Number of DEX orders in dex_orders table';
