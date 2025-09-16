-- Initial database schema for charms_indexer
-- This file is executed automatically when PostgreSQL container starts

-- Create bookmark table
CREATE TABLE IF NOT EXISTS bookmark (
    hash VARCHAR NOT NULL,
    height INTEGER NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    network VARCHAR NOT NULL DEFAULT 'Bitcoin-testnet4',
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    PRIMARY KEY (hash, network, blockchain)
);

-- Create charms table
CREATE TABLE IF NOT EXISTS charms (
    txid VARCHAR NOT NULL PRIMARY KEY,
    charmid VARCHAR NOT NULL,
    block_height INTEGER NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    date_created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    asset_type VARCHAR NOT NULL,
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    network VARCHAR NOT NULL DEFAULT 'testnet4'
);

-- Create transactions table
CREATE TABLE IF NOT EXISTS transactions (
    txid VARCHAR NOT NULL PRIMARY KEY,
    block_height INTEGER NOT NULL,
    ordinal BIGINT NOT NULL,
    raw JSONB NOT NULL DEFAULT '{}',
    charm JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR NOT NULL DEFAULT 'pending',
    confirmations INTEGER NOT NULL DEFAULT 0,
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    network VARCHAR NOT NULL DEFAULT 'testnet4'
);

-- Create summary table for optimized stats queries
CREATE TABLE IF NOT EXISTS summary (
    id SERIAL PRIMARY KEY,
    network VARCHAR NOT NULL,
    last_processed_block INTEGER NOT NULL DEFAULT 0,
    latest_confirmed_block INTEGER NOT NULL DEFAULT 0,
    total_charms BIGINT NOT NULL DEFAULT 0,
    total_transactions BIGINT NOT NULL DEFAULT 0,
    confirmed_transactions BIGINT NOT NULL DEFAULT 0,
    confirmation_rate INTEGER NOT NULL DEFAULT 0,
    nft_count BIGINT NOT NULL DEFAULT 0,
    token_count BIGINT NOT NULL DEFAULT 0,
    dapp_count BIGINT NOT NULL DEFAULT 0,
    other_count BIGINT NOT NULL DEFAULT 0,
    bitcoin_node_status VARCHAR NOT NULL DEFAULT 'unknown',
    bitcoin_node_block_count BIGINT NOT NULL DEFAULT 0,
    bitcoin_node_best_block_hash VARCHAR NOT NULL DEFAULT 'unknown',
    last_updated TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Create seaql_migrations table for migration tracking
CREATE TABLE IF NOT EXISTS seaql_migrations (
    version VARCHAR NOT NULL PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for bookmark table
CREATE INDEX IF NOT EXISTS bookmark_height ON bookmark (height);

CREATE INDEX IF NOT EXISTS bookmark_blockchain_network ON bookmark (blockchain, network);

CREATE INDEX IF NOT EXISTS idx_bookmark_network_height ON bookmark (network, height);

CREATE INDEX IF NOT EXISTS idx_bookmark_network_status ON bookmark (network, status, height);

-- Create indexes for charms table
CREATE INDEX IF NOT EXISTS charms_block_height ON charms (block_height);

CREATE INDEX IF NOT EXISTS charms_asset_type ON charms (asset_type);

CREATE INDEX IF NOT EXISTS charms_charmid ON charms (charmid);

CREATE INDEX IF NOT EXISTS charms_blockchain_network ON charms (blockchain, network);

CREATE INDEX IF NOT EXISTS idx_charms_network_block_height ON charms (network, block_height);

-- Create indexes for transactions table
CREATE INDEX IF NOT EXISTS transactions_block_height ON transactions (block_height);

CREATE INDEX IF NOT EXISTS transactions_blockchain_network ON transactions (blockchain, network);

CREATE INDEX IF NOT EXISTS idx_transactions_network_updated ON transactions (network, updated_at);

-- Create unique index for summary table
CREATE UNIQUE INDEX IF NOT EXISTS idx_summary_network ON summary (network);

-- Insert initial summary rows for both networks
INSERT INTO
    summary (
        network,
        last_updated,
        created_at,
        updated_at
    )
VALUES (
        'mainnet',
        NOW(),
        NOW(),
        NOW()
    )
ON CONFLICT (network) DO NOTHING;

INSERT INTO
    summary (
        network,
        last_updated,
        created_at,
        updated_at
    )
VALUES (
        'testnet4',
        NOW(),
        NOW(),
        NOW()
    )
ON CONFLICT (network) DO NOTHING;

-- Mark the initial migration as applied
INSERT INTO
    seaql_migrations (version)
VALUES (
        'm20250617_000001_create_complete_schema'
    )
ON CONFLICT (version) DO NOTHING;