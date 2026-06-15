-- Migration: m20260615_000001_charms_pk_with_app_id
-- Purpose: extend the `charms` primary key to support multi-token UTXOs.
-- Anomaly A5: a single Bitcoin output can hold N distinct charm tokens
-- (different app_ids at different app_indices in the same NormalizedCharms).
-- The old PK `(txid, vout)` allowed only one row per output, so the
-- second + third + ... tokens at the same vout were silently dropped on
-- `ON CONFLICT DO NOTHING` during `save_batch`. Meanwhile `stats_holders`
-- incremented for ALL parsed tokens, leading to ghost balances that no
-- corresponding charm row could later decrement on spend.
--
-- New PK `(txid, vout, app_id)` represents the true protocol semantics:
-- each (output, token) is its own row. `mark_charms_as_spent_batch`
-- still works against `(txid, vout)` because the WHERE clause now
-- matches every token at that vout — spending the UTXO marks all tokens
-- in it as spent.
--
-- Safe on existing data: current rows are unique on (txid, vout) by
-- definition (the old PK enforced it), so no row collides on the new
-- PK either. Past multi-token outputs already had their non-first
-- tokens dropped — that data is lost; only future writes will fully
-- track multi-token UTXOs. A one-off backfill from `transactions.raw`
-- can be added later if historical recovery is needed.

ALTER TABLE charms DROP CONSTRAINT charms_pkey;
ALTER TABLE charms ADD PRIMARY KEY (txid, vout, app_id);

INSERT INTO seaql_migrations (version)
VALUES ('m20260615_000001_charms_pk_with_app_id')
ON CONFLICT (version) DO NOTHING;
