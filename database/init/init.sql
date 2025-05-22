-- Create tables if they don't exist

-- Bookmark table
CREATE TABLE IF NOT EXISTS bookmark (
    hash VARCHAR NOT NULL,
    height INTEGER NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    last_updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    network VARCHAR NOT NULL DEFAULT 'Bitcoin-testnet4',
    blockchain VARCHAR NOT NULL DEFAULT 'Bitcoin',
    PRIMARY KEY (hash, network)
);

CREATE INDEX IF NOT EXISTS bookmark_height ON bookmark (height);

CREATE INDEX IF NOT EXISTS bookmark_blockchain_network ON bookmark (blockchain, network);

-- Charms table
CREATE TABLE IF NOT EXISTS charms (
    txid VARCHAR NOT NULL PRIMARY KEY,
    charmid VARCHAR NOT NULL,
    block_height INTEGER NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    date_created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    asset_type VARCHAR NOT NULL
);

CREATE INDEX IF NOT EXISTS charms_block_height ON charms (block_height);

CREATE INDEX IF NOT EXISTS charms_asset_type ON charms (asset_type);

CREATE INDEX IF NOT EXISTS charms_charmid ON charms (charmid);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    txid VARCHAR NOT NULL PRIMARY KEY,
    block_height INTEGER NOT NULL,
    ordinal BIGINT NOT NULL,
    raw JSONB NOT NULL DEFAULT '{}',
    charm JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS transactions_block_height ON transactions (block_height);

-- Migration tracking table
CREATE TABLE IF NOT EXISTS seaql_migrations (
    version VARCHAR NOT NULL PRIMARY KEY,
    applied_at BIGINT NOT NULL
);