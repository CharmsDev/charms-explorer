# Database Migration Guide

This guide explains how to create and execute database migrations for the Charms Explorer project.

## Overview

The project uses SeaORM migrations to manage database schema changes. Migrations are Rust files that define how to apply (`up`) and rollback (`down`) database changes.

## Migration File Structure

Migrations are located in: `/database/src/migration/`

Each migration file follows the naming convention:
- `m{YYYYMMDD}_{HHMMSS}_{description}.rs`
- Example: `m20250916_000001_create_assets_table.rs`

## Creating a New Migration

### 1. Create the Migration File

Create a new file in `/database/src/migration/` with the proper naming convention:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Your migration logic here
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rollback logic here
        Ok(())
    }
}

#[derive(DeriveIden)]
enum YourTable {
    Table,
    // Column definitions
}
```

### 2. Register the Migration

Add your migration to `/database/src/migration/mod.rs`:

```rust
mod m20250916_000001_create_assets_table; // Add this line

// And add to the migrations vector:
fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        // existing migrations...
        Box::new(m20250916_000001_create_assets_table::Migration), // Add this line
    ]
}
```

## Running Migrations

### Local Development (Docker)

1. **Ensure Docker container is running:**
   ```bash
   # Check if charms-postgres container is running
   docker ps | grep charms-postgres
   ```

2. **Set up environment variables:**
   ```bash
   cd database
   cp .env.example .env
   # Edit .env with your local database credentials
   ```

3. **Run migrations:**
   ```bash
   cd database
   cargo run
   ```

   This will:
   - Create the database if it doesn't exist
   - Run all pending migrations
   - Display detailed logging of the process

### Alternative: Direct Migration Command

You can also run migrations using the migrate command:

```bash
cd database
cargo run migrate
```

### Production Environment

For production deployments, use the production migration script:

```bash
cd database/scripts
./migrate_production.sh
```

**Note:** Ensure production environment variables are properly configured before running.

## Database Configuration

The database connection is configured via environment variables:

```env
POSTGRES_USER=ch4rm5u53r
POSTGRES_PASSWORD=your_password
POSTGRES_DB=charms_indexer
DATABASE_URL=postgres://ch4rm5u53r:your_password@localhost:6003/charms_indexer
RUST_LOG=info
```

## Docker Database Setup

The local PostgreSQL database runs in Docker:
- Container name: `charms-postgres`
- Port mapping: `6003:5432` (local:container)
- Database name: `charms_indexer`

## Troubleshooting

### Common Issues

1. **Connection refused:**
   - Ensure Docker container is running
   - Check port mapping (6003:5432)
   - Verify database credentials

2. **Authentication failed:**
   - Check username/password in .env file
   - Ensure database container uses same credentials

3. **Migration already applied:**
   - Migrations are tracked in `seaql_migrations` table
   - Each migration runs only once

### Checking Migration Status

Connect to the database to check applied migrations:

```sql
SELECT * FROM seaql_migrations ORDER BY applied_at;
```

## Example: Assets Table Migration

The assets table migration (`m20250916_000001_create_assets_table.rs`) demonstrates:

- Creating a table with multiple columns
- Setting up primary and foreign keys
- Creating indexes for performance
- Using proper data types (JSON, timestamps, etc.)

Key features of the assets table:
- `app_id`: Primary index (unique identifier, can start with n/, t/, etc.)
- `txid` + `vout_index`: UTXO reference
- `charm_id`: Reference to charms table
- `block_height`: Block information
- `data`: JSON field for flexible data storage
- `asset_type`, `blockchain`, `network`: Classification fields

## Best Practices

1. **Always include both `up` and `down` methods**
2. **Use `if_not_exists()` for tables and indexes**
3. **Create appropriate indexes for query performance**
4. **Use proper data types (timestamps with timezone, JSON, etc.)**
5. **Test migrations on local environment first**
6. **Keep migrations small and focused**
7. **Document complex migration logic**

## Migration Rollback

To rollback migrations (use with caution):

```bash
cd database
cargo run -- migrate down
```

**Warning:** Rollbacks can cause data loss. Always backup production data before rolling back.