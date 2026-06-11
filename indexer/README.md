# Charms Indexer

Bitcoin indexer for the Charms protocol. Watches mainnet and testnet4 for
charm transactions, persists them to Postgres, and serves them via the
sibling `api/` crate.

---

## Quick start

```bash
make dev          # start the indexer in DEBUG mode
make test         # unit + integration tests (Docker required)
make test-unit    # unit tests only (no Docker)
make lint         # cargo clippy --all-targets
make coverage     # generates target/llvm-cov/html/index.html
make migrate      # apply pending schema migrations
make build        # release binary at target/release/charms-indexer
make stop         # kill the dev process
```

---

## Architecture (at a glance)

```
src/
├── application/indexer/
│   ├── bitcoin_processor.rs   live + reindex driver
│   ├── network_manager.rs     supervises per-network processors
│   ├── supervisor.rs          restarts panicked workers with backoff
│   ├── block/                 per-block pipeline (detect → save → spent → stats)
│   └── mempool/               mempool polling pipeline
│       ├── processor.rs       orchestrator (slim)
│       ├── dex_persistence.rs DEX order CRUD
│       ├── spend_extraction.rs  parse tx inputs → mempool_spends
│       ├── cleanup.rs         purge stale entries (24h)
│       └── reconcile.rs       drop side-effects of evicted txs (every 30s)
├── domain/
│   ├── models/                pure data
│   └── services/              tx_analyzer, native_charm_parser, dex/, address_extractor
├── infrastructure/
│   ├── bitcoin/               RPC client + provider abstraction
│   ├── cardano/metadata.rs    CIP-68 metadata fetch (Koios)
│   └── persistence/           entities + repositories + DbPool
└── utils/
    ├── logging.rs             tracing subscriber + log bridge
    └── metrics.rs             Prometheus exporter
```

---

## Operational workflows

### 1. Apply schema migrations

The indexer ships with a `migrate` binary that bundles every SQL file in
`../database/migrations/` at compile time. It applies pending migrations
inside a Postgres transaction and records the version in `seaql_migrations`.

```bash
DATABASE_URL=postgres://... make migrate
```

Idempotent. Re-running is safe — already-applied versions are skipped.

To add a new migration:
1. Write `database/migrations/m{YYYYMMDD}_{NNNNNN}_{name}.sql`
2. Add a matching `include_str!` entry to `src/bin/migrate.rs`
3. Commit both files together

### 2. Deploy a new version

```bash
# from indexer/
fly deploy
```

Pre-deploy checklist:
- `make test` passes
- Any new migration is committed AND the corresponding `include_str!` entry is in `migrate.rs`
- Apply the migration **before** rolling the binary if it changes schema in
  a way the old binary cannot tolerate. Otherwise apply after.

Post-deploy: `fly logs -a charms-explorer-indexer --no-tail` should show
the block processor cycling and `Mempool cycle N` lines.

### 3. Rollback

The block processor and mempool processor are both idempotent against
the schema:
- `charms` saves use `ON CONFLICT (txid, vout) DO NOTHING`
- `stats_holders` gates updates on `last_updated_block < {block}` (audit N1)
- `summary` skips updates when `last_processed_block >= {block}` (audit N7)
- DEX activity rows are short-circuited if already present (audit N10)

This means re-deploying an older binary is safe **as long as the schema
is at-or-newer than what the binary expects**. Never roll the schema
back without coordinating with the binary.

If a deploy is unhealthy:
```bash
fly releases -a charms-explorer-indexer
fly deploy --image-label <previous>
```

### 4. Reindex from scratch

Last-resort operation. Walks every block from genesis. Takes hours.

```bash
# inside the container:
psql "$DATABASE_URL" -f scripts/clean_for_reindex.sql   # or the appropriate cleanup script
# then restart the indexer; it begins from genesis_block_height
```

The `block_status` table tracks per-block progress so a restart picks
up where it left off.

### 5. Diagnose a stuck or slow indexer

1. **Check the current head** versus the chain tip:
   ```sql
   SELECT MAX(block_height) FROM transactions WHERE network = 'mainnet';
   ```
   Compare against `bitcoin-cli getblockcount`.

2. **Tail the logs** with structured spans (T3.7):
   ```bash
   fly logs -a charms-explorer-indexer --no-tail \
     | grep -E 'block|mempool|⚠️|❌'
   ```
   Each `block: …` span carries `network` and `height` fields.

3. **Scrape metrics**:
   ```bash
   curl http://localhost:9000/metrics
   ```
   Key series:
   - `indexer_blocks_processed_total{network}` — should grow with the chain
   - `indexer_current_height{network}` — gauge of the highest processed block
   - `indexer_block_processing_duration_seconds_bucket{network}` — histogram
   - `indexer_mempool_size{network}` — gauge of mempool size as the indexer sees it
   - `indexer_charms_detected_total{network,asset_type}` — charm-flow rate

4. **Mempool processor health**: a missing `Mempool cycle …` line for
   more than a minute means the processor panicked. The supervisor (T3.1)
   will restart it with a 30s backoff and you'll see an
   `[mempool/…] supervised task panicked (restart #N).` line.

5. **Graceful shutdown**: `Ctrl+C` (or `SIGTERM` on Fly) fires the
   cancellation token. The mempool processor finishes its current cycle
   and exits; block processors are aborted (`stop_all` timeout: 30s).

### 6. Local development against a real node

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/charms
export BITCOIN_MAINNET_RPC_HOST=...
export BITCOIN_MAINNET_RPC_USERNAME=...
export BITCOIN_MAINNET_RPC_PASSWORD=...
export BITCOIN_MAINNET_RPC_PORT=8332
export ENABLE_BITCOIN_MAINNET=true
export RUST_LOG=info,sqlx=warn
make dev
```

`METRICS_PORT=0` disables the Prometheus exporter (useful for tests).

---

## Configuration

Read from environment variables on startup. The full surface lives in
`src/config/mod.rs`. The most important ones:

| Variable | Purpose | Default |
|---|---|---|
| `DATABASE_URL` | Postgres connection string | — (required) |
| `BITCOIN_MAINNET_RPC_HOST` / `_PORT` / `_USERNAME` / `_PASSWORD` | mainnet RPC | — |
| `BITCOIN_TESTNET4_RPC_HOST` / `_PORT` / `_USERNAME` / `_PASSWORD` | testnet4 RPC | — |
| `ENABLE_BITCOIN_MAINNET` | start the mainnet processor | `false` |
| `ENABLE_BITCOIN_TESTNET4` | start the testnet4 processor | `false` |
| `RUST_LOG` | log filter (env_logger / tracing-subscriber syntax) | `info,sqlx=warn` |
| `METRICS_PORT` | Prometheus exporter port; `0` to disable | `9000` |
| `PROCESS_INTERVAL_MS` | sleep between block-processor cycles | `2000` |

---

## Testing

| Layer | Where | How |
|---|---|---|
| Unit | inline `#[cfg(test)]` in each module | `make test-unit` |
| Integration | `tests/it_*.rs` against ephemeral Postgres (testcontainers) | `make test` (needs Docker) |
| Coverage | `cargo llvm-cov` | `make coverage` |
| CI | `.github/workflows/indexer-ci.yml` | runs on push/PR touching `indexer/**` |

Bundled fixtures live in `tests/fixtures/`. The schema applied to the
ephemeral container is `tests/fixtures/schema.sql`, kept in sync with
the entity definitions (NOT auto-derived).

---

## Known limitations

Tracked in the `_rjj/issues/` notes (gitignored, internal). Highlights:

- **N6 cross-check Cardano**: the ADA→BTC claim heuristic flags any spend
  of a known beam-out tx as a beam-in. Refining requires cross-referencing
  Cardano transactions, which the indexer cannot do today.
- **Mempool propagation**: large-OP_RETURN txs (charm spells) do not
  propagate through standard Bitcoin Core peers. The wallet's broadcast
  path goes through Maestro + mempool.space; the indexer-side concern is
  only to keep the ghost mempool entries from confusing the API.
