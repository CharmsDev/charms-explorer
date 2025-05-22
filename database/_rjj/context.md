# Charms Explorer Database Service

This document provides information about the database service for the Charms Explorer project, including deployment instructions, connection details, and how to link it with other services.

## Overview

The database service is a dedicated PostgreSQL database deployed on fly.io. It's designed to be used by both the indexer and API services, providing a centralized database for the entire application.

## Deployment

### Prerequisites

- [flyctl](https://fly.io/docs/hands-on/install-flyctl/) installed and authenticated
- Access to the Charms Explorer project on fly.io

### Deployment Steps

1. Navigate to the database directory:
   ```bash
   cd database
   ```

2. Create a `.env` file with the database credentials:
   ```bash
   cp .env.example .env
   ```

3. Update the `.env` file with the following credentials:
   ```
   POSTGRES_USER=ch4rm5u53r
   POSTGRES_PASSWORD=8f7d56a1e2c9b3f4d6e8a7c5
   POSTGRES_DB=charms_indexer
   DATABASE_URL=postgres://ch4rm5u53r:8f7d56a1e2c9b3f4d6e8a7c5@localhost:5432/charms_indexer
   ```

4. Deploy the database service to fly.io:
   ```bash
   fly deploy
   ```

## Linking with Other Services

To link the database service with the indexer and API services, you need to update the `DATABASE_URL` environment variable in each service to point to the database service.

### Internal Connection URL

When connecting from another fly.io application, use the internal network hostname:

```
postgres://ch4rm5u53r:8f7d56a1e2c9b3f4d6e8a7c5@charms-explorer-database.internal:5432/charms_indexer
```

### Updating the Indexer Service

```bash
cd ../indexer
fly secrets set DATABASE_URL="postgres://ch4rm5u53r:8f7d56a1e2c9b3f4d6e8a7c5@charms-explorer-database.internal:5432/charms_indexer" --app charms-explorer-indexer
```

### Updating the API Service

```bash
cd ../api
fly secrets set DATABASE_URL="postgres://ch4rm5u53r:8f7d56a1e2c9b3f4d6e8a7c5@charms-explorer-database.internal:5432/charms_indexer" --app charms-explorer-api
```

## Database Credentials

- **Username**: ch4rm5u53r
- **Password**: 8f7d56a1e2c9b3f4d6e8a7c5
- **Database Name**: charms_indexer
- **Port**: 5432

## VM Configuration

The database service is deployed on a VM with the following specifications:
- Memory: 10GB
- CPU: 1 shared CPU
- Persistent volume: Mounted at `/data`

## Migrations

Migrations are automatically run when the database service is deployed, using the `release_command` specified in the `fly.toml` file:

```toml
[deploy]
  release_command = "charms-database migrate"
```

## Monitoring and Management

### Checking Status

```bash
fly status --app charms-explorer-database
```

### Viewing Logs

```bash
fly logs --app charms-explorer-database
```

### Accessing the Database Console

```bash
fly ssh console --app charms-explorer-database
psql -U ch4rm5u53r -d charms_indexer
```

## Backup and Restore

### Creating a Backup

```bash
fly ssh console --app charms-explorer-database
pg_dump -U ch4rm5u53r -d charms_indexer > /data/backup.sql
```

### Restoring from a Backup

```bash
fly ssh console --app charms-explorer-database
psql -U ch4rm5u53r -d charms_indexer < /data/backup.sql
```

## Troubleshooting

If you encounter issues connecting to the database from other services, check the following:

1. Ensure the `DATABASE_URL` environment variable is correctly set in the service
2. Verify that the database service is running: `fly status --app charms-explorer-database`
3. Check the database service logs for any errors: `fly logs --app charms-explorer-database`
4. Ensure the services are in the same region or have [regional peering](https://fly.io/docs/reference/regions/) enabled
