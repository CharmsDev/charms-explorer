.PHONY: setup build run start stop migrate reset status clean deploy

# Default target
all: setup build

# Setup environment
setup:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo ".env file created from .env.example"; \
	fi

# Build the application
build:
	cargo build

# Run the application
run:
	cargo run

# Start the database container
start:
	docker-compose up -d
	@echo "Database started on port 5432"

# Stop the database container
stop:
	docker-compose down

# Run database migrations
migrate:
	cargo run -- migrate

# Reset the database
reset:
	cargo run -- reset

# Show database status
status:
	cargo run -- status

# Create a new database
create:
	cargo run -- create

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Deploy to fly.io
deploy:
	@echo "Deploying database service to fly.io..."
	fly deploy
	@echo "Database service deployed successfully!"

# Help
help:
	@echo "Available targets:"
	@echo "  setup        - Create .env file from .env.example if it doesn't exist"
	@echo "  build        - Build the application"
	@echo "  run          - Run the application"
	@echo "  start        - Start the database container"
	@echo "  stop         - Stop the database container"
	@echo "  migrate      - Run database migrations"
	@echo "  reset        - Reset the database"
	@echo "  status       - Show database status"
	@echo "  create       - Create a new database"
	@echo "  clean        - Clean build artifacts"
	@echo "  deploy       - Deploy to fly.io"
