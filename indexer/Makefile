# Charms Indexer Makefile

# -----------------------------------------------
# 1. LOCAL DATABASE OPERATIONS
# -----------------------------------------------

# Start the local PostgreSQL database
.PHONY: db-start
db-start:
	@echo "🔍 Starting local PostgreSQL database..."
	docker-compose up -d postgres
	@echo "✅ Database started with secure credentials from .env file"

# Stop the local PostgreSQL database
.PHONY: db-stop
db-stop:
	@echo "🛑 Stopping local PostgreSQL database..."
	docker-compose down

# Run database migrations on local database
.PHONY: migrate
migrate: db-start
	@echo "🔄 Running database migrations on local database..."
	cargo run --bin migrate

# Reset local database (stop, remove volumes, start fresh)
.PHONY: db-reset
db-reset:
	@echo "🧹 Resetting local database..."
	docker-compose down -v
	docker-compose up -d postgres
	@echo "✅ Database reset. You may need to run 'make migrate' to recreate tables."


# -----------------------------------------------
# 2. RUST CODE OPERATIONS
# -----------------------------------------------

# Run the indexer
.PHONY: run
run:
	@echo "🚀 Running indexer..."
	cargo run --bin charms-indexer

# Build the indexer
.PHONY: build
build:
	@echo "🔨 Building indexer..."
	cargo build

# Clean build artifacts
.PHONY: clean
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

# Review code
.PHONY: check
check:
	@echo "🔍 Checking code..."
	cargo check
