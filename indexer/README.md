# Charms Indexer

High-performance, parallel Bitcoin indexer for the Charms protocol built in Rust.

## Architecture & Performance

- **Language**: Rust
- **Runtime**: Tokio async runtime optimized for high concurrency.
- **Parallel Pipeline**: Implements a multi-stage parallel processing pipeline for block fetching and transaction analysis, maximizing throughput across all CPU cores.
- **Non-Blocking Architecture**: CPU-intensive tasks (such as Charm parsing via the native `charms-client`) are offloaded to dedicated threads using `spawn_blocking`, ensuring the I/O loop remains lightning fast.
- **Database**: PostgreSQL with optimized batch ingestion.

## Configuration

Environment variables (see `.env` file):

- `DATABASE_URL` - PostgreSQL connection
- `BITCOIN_RPC_URL`, `BITCOIN_RPC_USER`, `BITCOIN_RPC_PASS` - Bitcoin Core RPC
- `INDEXER_BATCH_SIZE` - Parallel transaction processing (tune for CPU/RAM) 500 local - 50 remote

## Running

```bash
make dev      # Development mode
make release  # Production mode (optimized)
```
