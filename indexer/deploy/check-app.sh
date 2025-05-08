#!/bin/bash

# check-app.sh
# Checks if an app exists on Fly.io
# Run from: /path/to/charms-explorer/indexer

# Exit on error, treat unset variables as errors
set -eu

echo "🔍 Checking if app exists on Fly.io..."

# Get the app name from fly.toml
if [ -f "fly.toml" ]; then
    app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-indexer")
    echo "📋 App name from fly.toml: $app_name"
else
    echo "❌ Error: fly.toml not found."
    exit 1
fi

# Check if the app exists
echo "🔍 Checking if app $app_name exists..."
if flyctl status --app "$app_name" &>/dev/null; then
    echo "✅ App $app_name exists on Fly.io."
    echo "   You should use the redeploy.sh script instead of deploy.sh."
    echo "   Run: make redeploy"
else
    echo "❌ App $app_name does not exist on Fly.io."
    echo "   You should use the deploy.sh script, but with an existing database."
    echo "   When prompted to create a new database, select 'n' and enter the existing database name."
    echo "   Run: make deploy"
fi

# Check if the database exists
read -p "Enter organization (default: charms-inc): " org
org=${org:-charms-inc}
read -p "Enter database name (default: charms-indexer-db): " db_name
db_name=${db_name:-charms-indexer-db}

echo "🔍 Checking if database $db_name exists in organization $org..."
if flyctl apps list --org "$org" | grep -q "$db_name"; then
    echo "✅ Database $db_name exists in organization $org."
else
    echo "❌ Database $db_name does not exist in organization $org."
    echo "   Available apps in organization $org:"
    flyctl apps list --org "$org"
fi
