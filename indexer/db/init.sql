-- Charms Indexer Database Schema

-- Table to store the last processed block
CREATE TABLE bookmark (
    hash CHARACTER VARYING PRIMARY KEY,
    height INTEGER NOT NULL
);
CREATE INDEX bookmark_height ON bookmark(height);

-- Table to store charm transactions
CREATE TABLE charms (
    txid CHARACTER VARYING PRIMARY KEY,
    charmid CHARACTER VARYING NOT NULL,
    block_height INTEGER NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    date_created TIMESTAMP NOT NULL DEFAULT NOW(),
    asset_type CHARACTER VARYING NOT NULL
);
CREATE INDEX charms_block_height ON charms(block_height);
CREATE INDEX charms_asset_type ON charms(asset_type);
CREATE INDEX charms_charmid ON charms(charmid);

-- Table to store raw transactions
CREATE TABLE transactions (
    txid CHARACTER VARYING PRIMARY KEY,
    block_height INTEGER NOT NULL,
    ordinal BIGINT NOT NULL,
    raw JSONB NOT NULL DEFAULT '{}',
    charm JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX transactions_block_height ON transactions(block_height);
