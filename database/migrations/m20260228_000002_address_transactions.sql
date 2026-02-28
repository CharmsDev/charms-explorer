-- Migration: Create address_transactions table for BTC transaction history
-- This table stores the transaction history for monitored addresses.
-- Populated by: API seeding (bb_getAddress from QuickNode) + Indexer (real-time block/mempool)

CREATE TABLE IF NOT EXISTS address_transactions (
    txid          VARCHAR(64)  NOT NULL,
    address       VARCHAR(62)  NOT NULL,
    network       VARCHAR(10)  NOT NULL DEFAULT 'mainnet',
    direction     VARCHAR(4)   NOT NULL, -- 'in' or 'out'
    amount        BIGINT       NOT NULL, -- sats (always positive)
    fee           BIGINT       NOT NULL DEFAULT 0,
    block_height  INTEGER,               -- NULL = unconfirmed/mempool
    block_time    BIGINT,                -- unix timestamp
    confirmations INTEGER      NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (txid, address, network)
);

-- Index for fast lookup by address
CREATE INDEX IF NOT EXISTS idx_address_transactions_address
    ON address_transactions (address, network);

-- Index for ordering by block time
CREATE INDEX IF NOT EXISTS idx_address_transactions_block_time
    ON address_transactions (address, network, block_time DESC);
