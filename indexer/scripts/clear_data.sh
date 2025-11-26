#!/bin/bash

# Script to clear all data from database tables while keeping the structure intact
# This is SAFE - it only deletes data, not tables

# Set database connection parameters
DB_HOST="localhost"
DB_PORT="8003"
DB_USER="charms_user"
DB_PASSWORD="charms_password"
DB_NAME="charms_indexer"

echo "🧹 Clearing all data from database tables..."
echo "⚠️  This will delete all data but keep table structure intact"
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

# Clear all data from tables (order matters due to foreign keys)
echo "Clearing data from all tables..."

PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
DELETE FROM assets;
DELETE FROM likes; 
DELETE FROM bookmark;
DELETE FROM charms;
DELETE FROM transactions;
DELETE FROM summary;
"

if [ $? -eq 0 ]; then
    echo "✅ All data cleared successfully!"
    echo ""
    echo "📊 Verifying tables are empty..."
    
    PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
    SELECT 'assets' as table_name, COUNT(*) as row_count FROM assets
    UNION ALL
    SELECT 'charms', COUNT(*) FROM charms
    UNION ALL
    SELECT 'transactions', COUNT(*) FROM transactions
    UNION ALL
    SELECT 'bookmark', COUNT(*) FROM bookmark
    UNION ALL
    SELECT 'likes', COUNT(*) FROM likes
    UNION ALL
    SELECT 'summary', COUNT(*) FROM summary;
    "
    
    echo ""
    echo "🎉 Data clearing completed! Tables are ready for fresh indexing."
else
    echo "❌ Error occurred while clearing data"
    exit 1
fi
