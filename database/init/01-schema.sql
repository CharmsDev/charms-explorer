-- Complete database schema for charms_indexer
-- This file contains the full schema with all migrations applied
-- Use this to recreate the database without running migrations

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

-- Create assets table (from migration m20250916_000001_create_assets_table)
CREATE TABLE IF NOT EXISTS assets (
    id SERIAL PRIMARY KEY,
    app_id VARCHAR NOT NULL,
    txid VARCHAR NOT NULL,
    vout_index INTEGER NOT NULL,
    charm_id VARCHAR NOT NULL,
    block_height INTEGER NOT NULL,
    date_created TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    data JSONB NOT NULL DEFAULT '{}',
    asset_type VARCHAR NOT NULL,
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    network VARCHAR NOT NULL DEFAULT 'testnet4',
    name VARCHAR,
    symbol VARCHAR,
    description TEXT,
    image_url VARCHAR,
    total_supply NUMERIC(30, 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create likes table (from migration m20250619_000001_create_likes_table)
CREATE TABLE IF NOT EXISTS likes (
    id SERIAL PRIMARY KEY,
    charm_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (charm_id, user_id)
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

-- Create indexes for assets table (from migration m20250916_000001_create_assets_table)
CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_app_id ON assets (app_id);

CREATE INDEX IF NOT EXISTS idx_assets_utxo ON assets (txid, vout_index);

CREATE INDEX IF NOT EXISTS idx_assets_charm_id ON assets (charm_id);

CREATE INDEX IF NOT EXISTS idx_assets_blockchain_network ON assets (blockchain, network);

CREATE INDEX IF NOT EXISTS idx_assets_block_height ON assets (block_height);

CREATE INDEX IF NOT EXISTS idx_assets_asset_type ON assets (asset_type);

-- Create indexes for likes table (from migration m20250619_000001_create_likes_table)
CREATE INDEX IF NOT EXISTS idx_likes_charm_id ON likes (charm_id);

CREATE INDEX IF NOT EXISTS idx_likes_user_id ON likes (user_id);

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

-- Create address_utxos table (wallet UTXO index by address)
CREATE TABLE IF NOT EXISTS address_utxos (
    txid VARCHAR(64) NOT NULL,
    vout INTEGER NOT NULL,
    network VARCHAR(10) NOT NULL DEFAULT 'mainnet',
    address VARCHAR(62) NOT NULL,
    value BIGINT NOT NULL,
    script_pubkey VARCHAR(140) NOT NULL DEFAULT '',
    block_height INTEGER,
    PRIMARY KEY (txid, vout, network)
);

-- Index for wallet lookups
CREATE INDEX IF NOT EXISTS idx_address_utxos_address ON address_utxos (address, network);

-- Index for block-level operations (reindex/rollback)
CREATE INDEX IF NOT EXISTS idx_address_utxos_block ON address_utxos (block_height, network);

-- Create monitored_addresses table (on-demand address monitoring)
CREATE TABLE IF NOT EXISTS monitored_addresses (
    address VARCHAR(62) NOT NULL,
    network VARCHAR(10) NOT NULL DEFAULT 'mainnet',
    source VARCHAR(20) NOT NULL DEFAULT 'api',
    seeded_at TIMESTAMPTZ,
    seed_height INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (address, network)
);

CREATE INDEX IF NOT EXISTS idx_monitored_addresses_network ON monitored_addresses (network);

-- Create mempool_spends table (tracks UTXOs being spent by unconfirmed mempool txs)
CREATE TABLE IF NOT EXISTS mempool_spends (
    spending_txid VARCHAR(64) NOT NULL,
    spent_txid VARCHAR(64) NOT NULL,
    spent_vout INTEGER NOT NULL,
    network VARCHAR(10) NOT NULL DEFAULT 'mainnet',
    detected_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (
        spent_txid,
        spent_vout,
        network
    )
);

CREATE INDEX IF NOT EXISTS idx_mempool_spends_spending ON mempool_spends (spending_txid, network);

CREATE INDEX IF NOT EXISTS idx_mempool_spends_detected ON mempool_spends (detected_at);

-- Mark all migrations as applied
INSERT INTO
    seaql_migrations (version)
VALUES (
        'm20250617_000001_create_complete_schema'
    ),
    (
        'm20250618_000001_create_summary_table'
    ),
    (
        'm20250619_000001_create_likes_table'
    ),
    (
        'm20250916_000001_create_assets_table'
    ),
    (
        'm20260218_000001_create_address_utxos_table'
    ),
    (
        'm20260220_000001_create_monitored_addresses_table'
    ),
    (
        'm20260221_000001_mempool_indexing'
    )
ON CONFLICT (version) DO NOTHING;