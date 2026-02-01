-- Migration 009: Create block_status table for unified indexer control
-- This replaces the simple bookmark system with granular block tracking

CREATE TABLE IF NOT EXISTS block_status (
    block_height INTEGER NOT NULL,
    network VARCHAR NOT NULL DEFAULT 'testnet4',
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    downloaded BOOLEAN NOT NULL DEFAULT false,
    processed BOOLEAN NOT NULL DEFAULT false,
    confirmed BOOLEAN NOT NULL DEFAULT false,
    block_hash VARCHAR,
    tx_count INTEGER DEFAULT 0,
    charm_count INTEGER DEFAULT 0,
    downloaded_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (
        block_height,
        network,
        blockchain
    )
);

-- Index for finding unprocessed blocks efficiently
CREATE INDEX IF NOT EXISTS idx_block_status_pending ON block_status (
    network,
    blockchain,
    downloaded,
    processed
)
WHERE
    downloaded = true
    AND processed = false;

-- Index for finding blocks to download
CREATE INDEX IF NOT EXISTS idx_block_status_to_download ON block_status (
    network,
    blockchain,
    downloaded
)
WHERE
    downloaded = false;

-- Comment: Usage
-- Indexer flow:
--   1. Download block → INSERT with downloaded=true, processed=false
--   2. Process block → UPDATE processed=true
--
-- Reindex flow:
--   1. Run reset script: UPDATE block_status SET processed=false; TRUNCATE stats_holders, etc.
--   2. Run normal indexer → processes all downloaded but unprocessed blocks