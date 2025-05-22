# Charms Indexer

A Bitcoin Testnet 4 block parser that indexes charm transactions.

## Features

- Monitors the Bitcoin Testnet 4 blockchain for new blocks
- Detects and indexes charm transactions
- Stores data in a PostgreSQL database
- Uses SeaORM for database operations

## Prerequisites

- Rust and Cargo
- Access to a PostgreSQL database
- Bitcoin Core node (Testnet 4)

## Setup

1. Clone the repository
2. Configure environment variables in `.env` file
3. Run the indexer

## Running the Indexer

```bash
make run
```

This will:
- Connect to the database
- Begin processing blocks and indexing charm transactions

## Database Schema

The indexer uses the following tables:

- `bookmark`: Tracks the last processed block
- `charms`: Stores charm transactions
- `transactions`: Stores raw transactions

## Development

### ORM Entities

The database entities are defined in the `src/infrastructure/persistence/entities` directory:

- `bookmark.rs`: Represents the bookmark table
- `charms.rs`: Represents the charms table
- `transactions.rs`: Represents the transactions table

### Adding a New Entity

1. Create a new entity file in `src/infrastructure/persistence/entities/`
2. Add the entity to `src/infrastructure/persistence/entities/mod.rs` and `src/infrastructure/persistence/entities/prelude.rs`

## Commands

- `make run`: Run the indexer
- `make build`: Build the indexer
- `make clean`: Clean build artifacts
- `make test`: Run tests
- `make check`: Check code
- `make setup`: Setup development environment

## Deployment

The indexer is deployed on Fly.io.

### Fly.io Commands

```bash
# start
flyctl scale count indexer=1 --app charms-explorer-indexer
# stop
flyctl scale count indexer=0 --app charms-explorer-indexer
# check
flyctl status --app charms-explorer-indexer
# set ENV var
flyctl secrets set BITCOIN_RPC_HOST=bitcoind-t4-test.fly.dev --app charms-explorer-indexer
