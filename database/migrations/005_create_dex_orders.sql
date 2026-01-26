-- Migration: Create dex_orders table for Charms Cast DEX orders
-- Date: 2026-01-26
-- Description: Stores detailed DEX order information from Charms Cast and other platforms

CREATE TABLE IF NOT EXISTS dex_orders (
    -- Primary key: order_id is txid:vout of the output containing the order
    order_id VARCHAR(80) PRIMARY KEY,
    
    -- Transaction reference
    txid VARCHAR(64) NOT NULL,
    vout INTEGER NOT NULL,
    block_height INTEGER,
    
    -- Platform identifier (cast, rift, etc.)
    platform VARCHAR(50) NOT NULL DEFAULT 'cast',
    
    -- Order details
    maker VARCHAR(100) NOT NULL,
    side VARCHAR(10) NOT NULL CHECK (side IN ('ask', 'bid')),
    exec_type VARCHAR(20) NOT NULL CHECK (exec_type IN ('all_or_none', 'partial')),
    
    -- Price as numerator/denominator
    price_num BIGINT NOT NULL,
    price_den BIGINT NOT NULL,
    
    -- Amounts
    amount BIGINT NOT NULL,           -- sats
    quantity BIGINT NOT NULL,         -- tokens (with decimals)
    filled_amount BIGINT DEFAULT 0,   -- sats filled
    filled_quantity BIGINT DEFAULT 0, -- tokens filled
    
    -- Asset being traded
    asset_app_id VARCHAR(255) NOT NULL,
    
    -- Scrolls address (if applicable)
    scrolls_address VARCHAR(100),
    
    -- Order status
    status VARCHAR(20) NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'partial', 'filled', 'cancelled')),
    
    -- Parent order for partial fills
    parent_order_id VARCHAR(80),
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Network info
    blockchain VARCHAR(50) NOT NULL DEFAULT 'Bitcoin',
    network VARCHAR(50) NOT NULL DEFAULT 'mainnet'
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_dex_orders_txid ON dex_orders(txid);
CREATE INDEX IF NOT EXISTS idx_dex_orders_maker ON dex_orders(maker);
CREATE INDEX IF NOT EXISTS idx_dex_orders_asset ON dex_orders(asset_app_id);
CREATE INDEX IF NOT EXISTS idx_dex_orders_status ON dex_orders(status);
CREATE INDEX IF NOT EXISTS idx_dex_orders_side ON dex_orders(side);
CREATE INDEX IF NOT EXISTS idx_dex_orders_platform ON dex_orders(platform);
CREATE INDEX IF NOT EXISTS idx_dex_orders_block ON dex_orders(block_height);
CREATE INDEX IF NOT EXISTS idx_dex_orders_parent ON dex_orders(parent_order_id);

-- Composite index for orderbook queries
CREATE INDEX IF NOT EXISTS idx_dex_orders_orderbook ON dex_orders(asset_app_id, side, status, price_num, price_den);

-- Comments
COMMENT ON TABLE dex_orders IS 'DEX orders from Charms Cast and other platforms';
COMMENT ON COLUMN dex_orders.platform IS 'DEX platform: cast, rift, etc.';
COMMENT ON COLUMN dex_orders.side IS 'Order side: ask (sell) or bid (buy)';
COMMENT ON COLUMN dex_orders.exec_type IS 'Execution type: all_or_none or partial';
COMMENT ON COLUMN dex_orders.price_num IS 'Price numerator (price = num/den sats per unit)';
COMMENT ON COLUMN dex_orders.price_den IS 'Price denominator';
COMMENT ON COLUMN dex_orders.parent_order_id IS 'Reference to parent order for partial fills';
