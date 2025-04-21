#!/bin/bash

# redeploy.sh
# Redeploys an existing charms-explorer-api application to Fly.io.
# Run from: /path/to/charms-explorer/api
# Use this script when you've already deployed the app once and just need to update it.

# Exit on error, treat unset variables as errors
set -eu

echo "🔄 Starting redeployment of charms-explorer-api to Fly.io..."

# Step 1: Verify prerequisites
echo "🔍 Step 1: Checking prerequisites..."
for cmd in flyctl docker; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "❌ Error: $cmd not found. Please install it."; exit 1; }
done
flyctl auth whoami >/dev/null 2>&1 || { echo "❌ Error: Not logged into Fly.io. Run 'flyctl auth login'."; exit 1; }
echo "✅ Prerequisites verified."

# Step 2: Get app name
if [ -f "fly.toml" ]; then
    # Try to extract app name from fly.toml
    app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-api")
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

# Step 4: Deploy application
echo "🚀 Step 4: Redeploying application..."
flyctl deploy --app "$app_name"

# Step 5: Verify deployment
echo "✅ Step 5: Deployment verification..."
flyctl status --app "$app_name"
echo "ℹ️ Monitor logs with: flyctl logs --app $app_name"

echo "🎉 Redeployment of charms-explorer-api successful!"
