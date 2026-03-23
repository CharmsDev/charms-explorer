-- Migration: m20260323_000001_tx_type_column
-- Add tx_type column to transactions table for pre-computed transaction classification

ALTER TABLE transactions ADD COLUMN IF NOT EXISTS tx_type VARCHAR(30);

CREATE INDEX IF NOT EXISTS idx_transactions_tx_type ON transactions(tx_type);

-- Backfill existing transactions from tags
UPDATE transactions SET tx_type = CASE
    WHEN tags LIKE '%create-ask%' THEN 'dex_create_ask'
    WHEN tags LIKE '%create-bid%' THEN 'dex_create_bid'
    WHEN tags LIKE '%fulfill-ask%' THEN 'dex_fulfill_ask'
    WHEN tags LIKE '%fulfill-bid%' THEN 'dex_fulfill_bid'
    WHEN tags LIKE '%cancel%' THEN 'dex_cancel'
    WHEN tags LIKE '%partial-fill%' THEN 'dex_partial_fill'
    WHEN tags LIKE 'beaming%' THEN 'beaming'
    WHEN tags LIKE '%bro-mint%' THEN 'bro_mint'
    WHEN tags LIKE '%bro-transfer%' THEN 'token_transfer'
    WHEN tags LIKE '%bro%' THEN 'token_transfer'
    ELSE 'spell'
END
WHERE tx_type IS NULL;
