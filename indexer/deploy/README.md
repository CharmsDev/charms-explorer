# Charms Indexer Deployment

This directory contains scripts for deploying the Charms Indexer. While the Fly.io deployment targets have been removed from the Makefile, these scripts are preserved for reference or future use.

## Deployment Scripts

- `check-app.sh` - Checks if the app exists on the deployment platform
- `deploy.sh` - Handles initial deployment
- `redeploy.sh` - Updates an existing deployment
- `set-env-vars.sh` - Sets environment variables for the deployment

## Manual Setup Instructions

Follow these steps to configure a deployment environment:

### 1. Install CLI Tools

- Download and install the appropriate CLI tools for your deployment platform
- Verify installation with version commands

### 2. Authentication

- Log in to your deployment platform
- Confirm authentication with identity verification commands

### 3. Select an Organization

- List available organizations
- Select the appropriate organization for your deployment

### 4. Set Up Database

- Create a database instance
- Configure the database with appropriate settings:
  - Name: Use a descriptive name (e.g., `charms-indexer-db`)
  - Organization: Choose your organization
  - Region: Select an appropriate region
  - Configuration: Select appropriate resources based on needs
- Verify database creation
- Test connection to the database

### 5. Prepare Environment Variables

Required environment variables:
- Database connection details
- Optional variables:
  - `BITCOIN_RPC_URL`
  - `BITCOIN_RPC_USER`
  - `BITCOIN_RPC_PASSWORD`
  - `API_KEY`

### Deployment Process

1. Update configuration variables in deployment scripts if needed
2. Run the appropriate deployment script from the project directory
