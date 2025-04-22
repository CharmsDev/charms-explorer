#!/bin/bash

# deploy.sh
# Automates initial deployment of charms-explorer-api to Fly.io with PostgreSQL.
# Run from: /path/to/charms-explorer/api
# For redeployment, use redeploy.sh instead.

# Exit on error, treat unset variables as errors
set -eu

echo "🚀 Starting initial deployment of charms-explorer-api to Fly.io..."

# Step 1: Verify prerequisites
echo "🔍 Step 1: Checking prerequisites..."
for cmd in flyctl docker; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "❌ Error: $cmd not found. Please install it."; exit 1; }
done
flyctl auth whoami >/dev/null 2>&1 || { echo "❌ Error: Not logged into Fly.io. Run 'flyctl auth login'."; exit 1; }
echo "✅ Prerequisites verified."

# Step 2: Check if fly.toml exists and handle app creation
if [ -f "fly.toml" ]; then
    echo "📋 Found existing fly.toml file."
    app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-api")
    echo "📋 App name from fly.toml: $app_name"
    
    # Check if the app actually exists on fly.io
    if flyctl status --app "$app_name" &>/dev/null; then
        echo "✅ App $app_name exists on Fly.io."
        read -p "Use existing app? (y/n): " use_existing
        if [[ "$use_existing" != "y" ]]; then
            echo "🚀 Creating new app configuration..."
            echo "This will create a new app on Fly.io and generate a new fly.toml file."
            echo "When prompted, select 'No' for deploying now, as we need to set up the database first."
            flyctl launch --no-deploy
            
            # Get the app name from the newly generated fly.toml
            app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-api")
            echo "📋 New app name from fly.toml: $app_name"
        else
            echo "✅ Using existing app: $app_name"
        fi
    else
        echo "⚠️ Warning: App $app_name defined in fly.toml does not exist on Fly.io yet."
        echo "   We need to create it first."
        
        read -p "Create app with name $app_name? (y/n): " create_app
        if [[ "$create_app" == "y" ]]; then
            echo "🚀 Creating new app with name $app_name..."
            flyctl apps create "$app_name" --org charms-inc
            echo "✅ App $app_name created on Fly.io."
        else
            echo "🚀 Creating new app with a different name..."
            echo "This will create a new app on Fly.io and generate a new fly.toml file."
            echo "When prompted, select 'No' for deploying now, as we need to set up the database first."
            flyctl launch --no-deploy
            
            # Get the app name from the newly generated fly.toml
            app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-api")
            echo "📋 New app name from fly.toml: $app_name"
        fi
    fi
else
    echo "🚀 Step 2: Launching the application with fly launch..."
    echo "This will create a new app on Fly.io and generate a fly.toml file."
    echo "When prompted, select 'No' for deploying now, as we need to set up the database first."
    flyctl launch --no-deploy
    
    # Get the app name from the generated fly.toml
    if [ -f "fly.toml" ]; then
        app_name=$(grep "app =" fly.toml | cut -d '"' -f2 || grep "app =" fly.toml | cut -d "'" -f2 || echo "charms-explorer-api")
        echo "📋 App name from fly.toml: $app_name"
    else
        read -p "Enter app name (default: charms-explorer-api): " app_name
        app_name=${app_name:-charms-explorer-api}
    fi
fi

# Step 3: Attach PostgreSQL database
echo "🔗 Step 3: Attaching database to application..."
read -p "Enter database name to attach (default: charms-indexer-db): " db_name
db_name=${db_name:-charms-indexer-db}

# Check if the database exists
if ! flyctl postgres list | grep -q "$db_name"; then
    echo "⚠️ Warning: Database $db_name does not exist on Fly.io."
    echo "   The API requires a PostgreSQL database to function."
    echo "   Please make sure the indexer database is already created."
    read -p "Do you want to continue without attaching a database? (y/n): " continue_without_db
    if [[ "$continue_without_db" != "y" ]]; then
        echo "❌ Deployment aborted. Please create the database first."
        exit 1
    fi
else
    # Attach the database
    flyctl postgres attach "$db_name" -a "$app_name" --yes
    echo "✅ Database attached."
    echo "   The DATABASE_URL environment variable has been automatically set."
fi

# Step 4: Configure environment variables
echo "🔧 Step 4: Setting environment variables..."

# Create a temporary file with all environment variables
echo "Setting all environment variables at once..."
cat > .env-deploy.tmp << EOF
PORT=3000
HOST=0.0.0.0
RUST_LOG=info
EOF

# Set all environment variables at once
flyctl secrets import --app "$app_name" < .env-deploy.tmp
rm .env-deploy.tmp
echo "✅ All environment variables set."

# Step 5: Deploy application
echo "🚀 Step 5: Deploying application..."
read -p "Do you want to deploy now? (y/n): " deploy_now
if [[ "$deploy_now" == "y" ]]; then
    flyctl deploy --app "$app_name"
    
    # Step 6: Verify deployment
    echo "✅ Step 6: Deployment verification..."
    flyctl status --app "$app_name"
    echo "ℹ️ Monitor logs with: flyctl logs --app $app_name"
    
    echo "🎉 Deployment of charms-explorer-api successful!"
else
    echo "Deployment skipped. You can deploy later with: flyctl deploy --app $app_name"
    echo "Or use redeploy.sh for subsequent deployments."
fi
