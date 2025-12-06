-- Migration: Add decimals field to assets table
-- [RJJ-DECIMALS] Support for dynamic decimal precision based on NFT metadata
-- Date: 2025-02-05

-- Add decimals column (default 8 for Bitcoin standard)
ALTER TABLE assets
ADD COLUMN IF NOT EXISTS decimals SMALLINT NOT NULL DEFAULT 8;

-- Add constraint to ensure decimals is between 0 and 18 (safety)
ALTER TABLE assets
ADD CONSTRAINT check_decimals_range CHECK (
    decimals >= 0
    AND decimals <= 18
);

-- Create index for faster queries by decimals
CREATE INDEX IF NOT EXISTS idx_assets_decimals ON assets (decimals);

-- Comment on column
COMMENT ON COLUMN assets.decimals IS '[RJJ-DECIMALS] Number of decimal places for token amounts. Default: 8 (Bitcoin standard). Max: 18 (safety).';