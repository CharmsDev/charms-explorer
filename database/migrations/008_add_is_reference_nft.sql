-- Migration: Add is_reference_nft flag to assets table
-- This flag marks NFTs that serve as metadata source for tokens (same hash, different prefix)
-- Reference NFTs should be hidden from the NFT list but accessible for token metadata lookup

ALTER TABLE assets ADD COLUMN IF NOT EXISTS is_reference_nft BOOLEAN DEFAULT FALSE;

-- Create index for efficient filtering of reference NFTs
CREATE INDEX IF NOT EXISTS idx_assets_is_reference_nft ON assets(is_reference_nft) WHERE is_reference_nft = TRUE;

-- Update existing NFTs that have associated tokens
-- An NFT is a reference if there exists a token with the same hash
UPDATE assets AS nft
SET is_reference_nft = TRUE
WHERE nft.asset_type = 'nft'
  AND EXISTS (
    SELECT 1 FROM assets AS token
    WHERE token.asset_type = 'token'
      AND SUBSTRING(token.app_id FROM 3) = SUBSTRING(nft.app_id FROM 3)
  );

COMMENT ON COLUMN assets.is_reference_nft IS 'True if this NFT is a reference NFT for a token (should be hidden from NFT list)';
