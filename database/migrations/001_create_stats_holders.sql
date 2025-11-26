-- [RJJ-STATS-HOLDERS] Migration: Create stats_holders table
-- This table maintains aggregated holder statistics per asset and address
-- Updated in real-time by the indexer when charms are created/spent

-- Create stats_holders table
CREATE TABLE IF NOT EXISTS stats_holders (
    id SERIAL PRIMARY KEY,
    app_id VARCHAR NOT NULL,
    address VARCHAR NOT NULL,
    total_amount BIGINT NOT NULL DEFAULT 0,
    charm_count INTEGER NOT NULL DEFAULT 0,
    first_seen_block INTEGER NOT NULL,
    last_updated_block INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Unique constraint: one row per (app_id, address) combination
    CONSTRAINT unique_app_id_address UNIQUE (app_id, address)
);

-- Create indexes for efficient queries
CREATE INDEX idx_stats_holders_app_id ON stats_holders (app_id);
CREATE INDEX idx_stats_holders_address ON stats_holders (address);
CREATE INDEX idx_stats_holders_amount ON stats_holders (app_id, total_amount DESC);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_stats_holders_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update updated_at
CREATE TRIGGER trigger_stats_holders_updated_at
    BEFORE UPDATE ON stats_holders
    FOR EACH ROW
    EXECUTE FUNCTION update_stats_holders_updated_at();

-- Display table structure
\d stats_holders;
