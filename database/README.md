# Charms Explorer Database Service

Standalone service for managing the Charms Explorer database. Handles database creation, migrations, and deployment.

## Quick Start

1. Copy `.env.example` to `.env` and configure
2. Start database: `make start`
3. Run migrations: `make migrate`

## Commands

- `make start/stop`: Start/stop database container
- `make migrate`: Run migrations
- `make deploy`: Deploy to fly.io
- `make link-indexer/link-api`: Link with other services

For detailed configuration, check the `.env` file.
