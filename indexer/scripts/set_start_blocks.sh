#!/bin/bash

# Script to set the starting block heights for mainnet and testnet4 indexing
# Usage: ./set_start_blocks.sh [mainnet_block] [testnet4_block]
# Example: ./set_start_blocks.sh 900000 42000

# Set database connection parameters
DB_HOST="localhost"
DB_PORT="8003"
DB_USER="charms_user"
DB_PASSWORD="charms_password"
DB_NAME="charms_indexer"

# Default values
DEFAULT_MAINNET_BLOCK=912774 # Pruned block start (912774 + 1)
DEFAULT_TESTNET4_BLOCK=100000 # september 2025

# Parse command line arguments
MAINNET_BLOCK=${1:-$DEFAULT_MAINNET_BLOCK}
TESTNET4_BLOCK=${2:-$DEFAULT_TESTNET4_BLOCK}

echo "🚀 Setting indexer start blocks..."
echo "📍 Mainnet will start from block: $MAINNET_BLOCK"
echo "📍 Testnet4 will start from block: $TESTNET4_BLOCK"
echo ""

# Test database connection
echo "Testing database connection..."
if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" &> /dev/null; then
    echo "✅ Connection successful!"
    echo ""
else
    echo "❌ Error: Could not connect to the database"
    exit 1
fi

# Show current bookmarks
echo "📖 Current bookmarks:"
PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT network, height, status FROM bookmark ORDER BY network;"
echo ""

# Delete existing bookmarks and insert new ones
echo "🔄 Updating bookmarks..."
PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
DELETE FROM bookmark;
INSERT INTO bookmark (hash, height, status, last_updated_at, network, blockchain) 
VALUES 
    ('genesis', $TESTNET4_BLOCK, 'pending', NOW(), 'testnet4', 'Bitcoin'),
    ('genesis', $MAINNET_BLOCK, 'pending', NOW(), 'mainnet', 'Bitcoin');
"

if [ $? -eq 0 ]; then
    echo "✅ Bookmarks updated successfully!"
    echo ""
    echo "📖 New bookmarks:"
    PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT network, height, status, last_updated_at FROM bookmark ORDER BY network;"
    echo ""
    echo "🎉 Indexer is ready to start from the specified blocks!"
    echo "💡 Restart the indexer to pick up the new bookmarks."
else
    echo "❌ Error occurred while updating bookmarks"
    exit 1
fi
