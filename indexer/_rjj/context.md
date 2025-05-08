# Charms Indexer Context

## Project Overview
The Charms Indexer is a Rust application that processes Bitcoin blocks to find and index charm transactions. It connects to a Bitcoin node, fetches block data, and identifies potential charm transactions. It then fetches additional data from an API and stores the results in a PostgreSQL database.

Code is reading all transactions in every block, but it's only processing in detail the transactions that are identified as potential "charms".
Process:
1. The block processor retrieves each block by height
2. It iterates through every transaction in the block using for (tx_pos, tx) in block.txdata.iter().enumerate()
3. For each transaction, it gets the raw transaction data
4. It then checks if the transaction could be a charm by looking for the "spell" marker in the transaction data
5. Only transactions that contain this marker are processed further and stored in the database
The charm detection is done in the could_be_charm function, which simply looks for the ASCII string "spell" in the transaction data. If found, the transaction is considered a potential charm and undergoes further processing.

## Key Components
- **Block Processor**: Processes Bitcoin blocks and identifies potential charm transactions
- **Charm Detector**: Detects if a transaction could be a charm
- **Charms Fetch**: Fetches charm data from an external API
- **Database**: Stores transaction and charm data
- **Diagnostic Service**: Provides information about the indexer's status and statistics
- **Web Server**: Serves the diagnostic API and dashboard

## Database Structure
- **transactions**: Stores all transactions that were detected as potential charms
- **charms**: Stores only valid charms with actual data
- **bookmark**: Tracks the last processed block and includes a timestamp for monitoring

## Recent Changes
- Added timestamp tracking to the bookmark table to monitor the indexer's activity
- Created a diagnostic service to provide information about the indexer's status
- Added a web server to expose the diagnostic API
- Created a dashboard to visualize the indexer's status and statistics
- Added a button to the webapp header to access the status dashboard

## Useful Commands

### Docker Commands
```bash
# Start the database
docker-compose up -d postgres

# Stop the database
docker-compose down

# Reset the database (remove all data)
docker-compose down -v
docker-compose up -d postgres

# Execute SQL commands in the database
docker exec -it charms-indexer-db psql -U ch4rm5u53r -d charms_indexer -c "SELECT COUNT(*) FROM charms;"
docker exec -it charms-indexer-db psql -U ch4rm5u53r -d charms_indexer -c "SELECT COUNT(*) FROM transactions;"
docker exec -it charms-indexer-db psql -U ch4rm5u53r -d charms_indexer -c "TRUNCATE bookmark, charms, transactions;"
```

### API Endpoints
```bash
# Get health status
curl http://localhost:5002/api/health

# Get diagnostic information
curl http://localhost:5002/api/diagnostic

# Get indexer status
curl http://localhost:5002/status
# or with api prefix
curl http://localhost:5002/api/status
```

### Makefile
The project includes a Makefile with various commands for database operations, running the indexer, building the project, and deployment operations. Use `make help` or check the Makefile for available commands.

### SQL Queries
```sql
-- Count charms
SELECT COUNT(*) FROM charms;

-- Count transactions
SELECT COUNT(*) FROM transactions;

-- View charm data
SELECT * FROM charms LIMIT 10;

-- View transaction data
SELECT * FROM transactions LIMIT 10;

-- View bookmark data
SELECT * FROM bookmark ORDER BY last_updated_at DESC LIMIT 1;

-- Clear all data
TRUNCATE bookmark, charms, transactions;
```

## Notes
- The indexer now tracks the timestamp of the last processed block, which can be used to monitor its activity
- The diagnostic service provides information about the indexer's status, including the last processed block, the latest confirmed block, and the time since the last update
- The API server runs on port 5002 by default, configurable via the API_PORT environment variable
- The webapp now includes a status page at /status that displays the indexer's status and statistics
- The webapp header includes a button to access the status page
