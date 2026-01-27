-- Migration: Add verified column to charms table
-- This column tracks whether the spell proof has been verified during indexing

-- Add verified column with default true (existing charms were verified during extraction)
ALTER TABLE charms ADD COLUMN IF NOT EXISTS verified BOOLEAN NOT NULL DEFAULT true;

-- Create index for filtering by verification status
CREATE INDEX IF NOT EXISTS idx_charms_verified ON charms(verified);
