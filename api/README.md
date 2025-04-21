# Charms Explorer API

This API provides endpoints to access charms data from the Charms Explorer database.

## Setup

### Running Locally

1. Make sure you have the PostgreSQL database running with the Charms Explorer schema:

```bash
cd ../indexer
docker-compose up -d
```

2. Install dependencies and build the API:

```bash
cargo build
```

3. Run the API server:

```bash
cargo run
```

The API will be available at `http://localhost:3000`.

### Running with Docker

1. Build and run the API using Docker Compose:

```bash
docker-compose up -d
```

This will start both the PostgreSQL database and the API server. The API will be available at `http://localhost:3000`.

### Using Makefile

The project includes a Makefile with common commands:

```bash
# Build the API
make build

# Run the API
make run

# Run the API in release mode
make run-release

# Clean build artifacts
make clean

# Run tests
make test

# Check code without building
make check

# Format code
make fmt

# Lint code
make lint

# Build documentation
make docs
```

## Environment Variables

The API can be configured using the following environment variables:

- `PORT`: The port to listen on (default: 3000)
- `HOST`: The host to bind to (default: 0.0.0.0)
- `DATABASE_URL`: The PostgreSQL connection string (default: postgres://charms:charms@localhost:5432/charms_indexer)
- `RUST_LOG`: The log level (default: info)

## API Endpoints

### Health Check

```
GET /api/health
```

Returns the health status of the API.

**Response:**

```json
{
  "status": "ok"
}
```

### Get All Charms Numbers

```
GET /api/charms/numbers
```

Returns a list of all charm numbers (charmids).

**Query Parameters:**

- `type` (optional): Filter by asset type

**Response:**

```json
{
  "charm_numbers": ["charm-123abc", "charm-456def", ...]
}
```

### Get All Charms

```
GET /api/charms
```

Returns a list of all charms with their full data.

**Response:**

```json
{
  "charms": [
    {
      "txid": "123abc",
      "charmid": "charm-123abc",
      "block_height": 12345,
      "data": { ... },
      "date_created": "2025-04-15T23:00:00",
      "asset_type": "spell"
    },
    ...
  ]
}
```

### Get Charms by Type

```
GET /api/charms/by-type?type=spell
```

Returns a list of charms filtered by asset type.

**Query Parameters:**

- `type` (required): The asset type to filter by

**Response:**

```json
{
  "charms": [
    {
      "txid": "123abc",
      "charmid": "charm-123abc",
      "block_height": 12345,
      "data": { ... },
      "date_created": "2025-04-15T23:00:00",
      "asset_type": "spell"
    },
    ...
  ]
}
```

### Get Charm by TXID

```
GET /api/charms/:txid
```

Returns a single charm by its transaction ID.

**Path Parameters:**

- `txid`: The transaction ID of the charm

**Response:**

```json
{
  "txid": "123abc",
  "charmid": "charm-123abc",
  "block_height": 12345,
  "data": { ... },
  "date_created": "2025-04-15T23:00:00",
  "asset_type": "spell"
}
```

## Error Handling

All endpoints return appropriate HTTP status codes and error messages in case of failure.

**Example Error Response:**

```json
{
  "error": "Charm with txid 123abc not found"
}
