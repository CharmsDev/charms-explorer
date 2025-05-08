# Fly.io Manual Setup Instructions for Charms Indexer
# File: flyio_manual_setup_instructions.txt

Follow these steps to configure Fly.io before running `deploy.sh`:

### 1. Install Fly CLI
- Download and install `flyctl` from [Fly.io Docs](https://fly.io/docs/hands-on/install-flyctl/).
- Verify installation: `flyctl version`.

### 2. Authenticate with Fly.io
- Log in: `flyctl auth login` (or sign up: `flyctl auth signup`).
- Confirm: `flyctl auth whoami`.

### 3. Select an Organization
- List organizations: flyctl orgs list
Name                 Slug                 Type      
----                 ----                 ----      
Charms Inc           charms-inc           SHARED    
Ricart Juncadella    personal             PERSONAL 

Use:charms-inc

### 4. Set Up PostgreSQL Database
- Create the database: `flyctl postgres create`.
  - **App name**: Use `charms-indexer-db` (or note a custom name for the script).
  - **Organization**: Choose your organization.
  - **Region**: Select a region (e.g., `iad` for Ashburn, VA).
  - **Configuration**: Pick the free tier (`Development - Single node, 1x shared CPU, 256MB RAM, 1GB disk`).
  - **Scale to zero**: Select `No` for continuous operation.
- Verify: `flyctl apps list` (look for your database app).
- Test connection: `flyctl pg connect --app charms-indexer-db`.

### 5. Prepare Additional Environment Variables
- Identify optional variables (e.g., `BITCOIN_RPC_URL`, `BITCOIN_RPC_USER`, `BITCOIN_RPC_PASSWORD`, `API_KEY`) needed by your app.
- These will be set via the deployment script.

### Next Steps
- Update `deploy.sh` configuration variables if your database name, region, or organization differ.
- Run `deploy.sh` from your project directory.