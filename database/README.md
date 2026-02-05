# Database - Charms Explorer

## Production (Managed Postgres)

- **Type**: Fly.io Managed Postgres
- **Cluster ID**: `z7y24odjmppogqd1`
- **Name**: `charms-explorer-db`
- **Region**: sjc
- **Plan**: Basic (Shared x 2, 1 GB RAM)

### Quick Commands

```bash
# Open proxy tunnel
./scripts/db_connect.sh

# Connect (in another terminal)
PGPASSWORD='QdEj8kthuWMfIx4cHAmYvxek' psql -h localhost -p 16432 -U fly-user -d fly-db

# Test connection
./scripts/db_test.sh

# Run migration
./scripts/migrate_production.sh migrations/009_example.sql
```

### Import/Export

```bash
# Export from production
fly mpg proxy z7y24odjmppogqd1 --local-port 16432
PGPASSWORD='QdEj8kthuWMfIx4cHAmYvxek' pg_dump -h localhost -p 16432 -U fly-user -d fly-db -Fc > backup.dump

# Import to production
PGPASSWORD='QdEj8kthuWMfIx4cHAmYvxek' pg_restore -h localhost -p 16432 -U fly-user -d fly-db --clean --if-exists --no-owner --no-acl backup.dump
```

## Local Development

- **Container**: `charms-postgres`
- **Port**: `8003`
- **Database**: `charms_indexer`
- **User**: `charms_user`
- **Password**: `charms_password`

```bash
# Start local database
docker-compose up -d

# Connect
PGPASSWORD=charms_password psql -h localhost -p 8003 -U charms_user -d charms_indexer
```

## Migrations

All migrations are in `migrations/` folder. Apply in order:

1. Local first: `docker exec -i charms-postgres psql -U charms_user -d charms_indexer < migrations/XXX.sql`
2. Then production: `./scripts/migrate_production.sh migrations/XXX.sql`

See `_rjj/credentials/production.md` for full credentials.
