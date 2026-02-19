# Charms Explorer API Makefile

PORT=8000

.PHONY: build makeDev dev clean test check install-watch

# Default target
all: build

# Build the API
build:
	cargo build

# Development server with port cleanup
makeDev:
	@echo "Killing any process using port $(PORT)..."
	@lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || true
	@echo "Starting API server with hot reload on port $(PORT)..."
	@echo "Loading environment variables from .env file..."
	@export $$(grep -v '^#' .env | grep -v '^\s*$$' | xargs) && cargo watch -x run

# Alias for makeDev
dev: makeDev

# Install cargo-watch for hot reloading
install-watch:
	cargo install cargo-watch

# Run the API in release mode
run-release:
	cargo run --release

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test

# Check code without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Lint code
lint:
	cargo clippy

# Build documentation
docs:
	cargo doc --no-deps --open
