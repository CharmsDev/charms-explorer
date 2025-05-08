#!/bin/bash

# set-env-vars.sh
# Sets environment variables for the charms-indexer application on Fly.io.
# Run from: /path/to/charms-explorer/indexer

# Exit on error, treat unset variables as errors
set -eu

echo "🔧 Setting environment variables for charms-indexer on Fly.io..."

# Step 1: Verify prerequisites
echo "🔍 Step 1: Checking prerequisites..."
for cmd in flyctl; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "❌ Error: $cmd not found. Please install it."; exit 1; }
done
flyctl auth whoami >/dev/null 2>&1 || { echo "❌ Error: Not logged into Fly.io. Run 'flyctl auth login'."; exit 1; }
echo "✅ Prerequisites verified."

# Step 2: Get app name
if [ -f "fly.toml" ]; then
    # Try to extract app name from fly.toml
    app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-indexer")
    read -p "Enter app name (default: $app_name): " input_app_name
    app_name=${input_app_name:-$app_name}
else
    echo "⚠️ Warning: fly.toml not found in current directory."
    read -p "Enter app name: " app_name
    if [ -z "$app_name" ]; then
        echo "❌ Error: App name is required."
        exit 1
    fi
fi

# Step 3: Verify app exists
echo "🔍 Step 3: Verifying app exists on Fly.io..."
if ! flyctl status --app "$app_name" &>/dev/null; then
    echo "❌ Error: App $app_name does not exist on Fly.io."
    echo "   If this is a new deployment, use deploy.sh instead."
    exit 1
fi
echo "✅ App $app_name exists on Fly.io."

# Step 4: Set environment variables
echo "🔧 Step 4: Setting environment variables..."
echo "Setting all environment variables at once..."
cat > .env-deploy.tmp << EOF
BITCOIN_RPC_HOST=bitcoind-t4-test.fly.dev
BITCOIN_RPC_PORT=48332
BITCOIN_RPC_USER=hello
BITCOIN_RPC_PASSWORD=world
BITCOIN_NETWORK=testnet4
CHARMS_API_URL=https://api-t4.charms.dev
BLOCKS_UNTIL_FINALIZED=0
PROCESS_BLOCK_INTERVAL_MS=120000
GENESIS_BLOCK_HASH=00000000338e09eb14a8df7f186c04db96c351b414fc186f4f4eea0c5941c1b2
GENESIS_BLOCK_HEIGHT=57604
TX_ORDINAL_BLOCK_HEIGHT_OFFSET=10000
RUST_LOG=info
EOF

# Set all environment variables at once
flyctl secrets import --app "$app_name" < .env-deploy.tmp
rm .env-deploy.tmp
echo "✅ All environment variables set."

echo "🎉 Environment variables for $app_name have been updated!"
