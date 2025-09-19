#!/bin/bash

# Script to set the starting block heights for mainnet and testnet4 indexing
# Usage: ./set_start_blocks.sh [mainnet_block] [testnet4_block]

# Database connection parameters
DB_HOST="localhost"
DB_PORT="8003"
DB_USER="charms_user"
DB_PASSWORD="charms_password"
DB_NAME="charms_indexer"

# Default values
DEFAULT_MAINNET_BLOCK=913848
DEFAULT_TESTNET4_BLOCK=100000

# Parse command line arguments
MAINNET_BLOCK=${1:-$DEFAULT_MAINNET_BLOCK}
TESTNET4_BLOCK=${2:-$DEFAULT_TESTNET4_BLOCK}

# Test database connection
if ! PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" &> /dev/null; then
    echo "❌ Error: Could not connect to the database"
    exit 1
fi

# Update bookmarks
PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
DELETE FROM bookmark;
INSERT INTO bookmark (hash, height, status, last_updated_at, network, blockchain) 
VALUES 
    ('genesis', $TESTNET4_BLOCK, 'pending', NOW(), 'testnet4', 'Bitcoin'),
    ('genesis', $MAINNET_BLOCK, 'pending', NOW(), 'mainnet', 'Bitcoin');
" &> /dev/null

if [ $? -eq 0 ]; then
    echo "✅ Indexer empezará desde:"
    echo "   • Mainnet: bloque $MAINNET_BLOCK"
    echo "   • Testnet4: bloque $TESTNET4_BLOCK"
else
    echo "❌ Error al actualizar bookmarks"
    exit 1
fi
