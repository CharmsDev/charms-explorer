make # Charms Indexer

A Bitcoin Testnet 4 block parser that indexes charm transactions.

## Features

- Monitors the Bitcoin Testnet 4 blockchain for new blocks
- Detects and indexes charm transactions
- Stores data in a PostgreSQL database
- Uses SeaORM for database operations and migrations

## Prerequisites

- Rust and Cargo
- Docker and Docker Compose (for PostgreSQL)
- Bitcoin Core node (Testnet 4)

## Setup

1. Clone the repository
2. Configure environment variables in `.env` file
3. Start the database and run migrations:

```bash
make setup
```

This will:
- Start the PostgreSQL database container
- Run database migrations to create the necessary tables

## Running the Indexer

```bash
make run-with-db
```

This will:
- Start the PostgreSQL database if not already running
- Run the indexer, which will automatically run migrations
- Begin processing blocks and indexing charm transactions

## Database Schema

The indexer uses the following tables:

- `bookmark`: Tracks the last processed block
- `charms`: Stores charm transactions
- `transactions`: Stores raw transactions

## Development

### Database Migrations

The project uses SeaORM for database operations and migrations. Migrations are defined in the `migration` crate.

To run migrations manually:

```bash
make migrate
```

To create a new migration:

1. Add a new migration file in `migration/src/` with a timestamp prefix
2. Update the `migration/src/lib.rs` file to include the new migration
3. Implement the `up` and `down` methods in your migration

### ORM Entities

The database entities are defined in the `src/entity` directory:

- `bookmark.rs`: Represents the bookmark table
- `charms.rs`: Represents the charms table
- `transactions.rs`: Represents the transactions table

### Adding a New Entity

1. Create a new entity file in `src/entity/`
2. Add the entity to `src/entity/mod.rs` and `src/entity/prelude.rs`
3. Create a migration to add the new table

## Commands

- `make run`: Run the indexer
- `make db-start`: Start the PostgreSQL database
- `make db-stop`: Stop the PostgreSQL database
- `make migrate`: Run database migrations
- `make run-with-db`: Start the database and run the indexer
- `make test-env`: Run the test environment
- `make build`: Build the indexer
- `make clean`: Clean build artifacts
- `make test`: Run tests
- `make check`: Check code
- `make setup`: Setup development environment

## RJJ-TODO
# start
flyctl scale count indexer=1 --app charms-explorer-indexer
# stop
flyctl scale count indexer=0 --app charms-explorer-indexer
# check
flyctl status --app charms-explorer-indexer
# set ENV var
flyctl secrets set BITCOIN_RPC_HOST=bitcoind-t4-test.fly.dev --app charms-explorer-indexer