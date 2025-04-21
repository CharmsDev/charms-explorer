# Charms Explorer API Makefile

.PHONY: build run dev clean test check install-watch

# Default target
all: build

# Build the API
build:
	cargo build

# Run the API
run:
	cargo run

# Run the API with hot reload
dev:
	@echo "Killing any process using port 5002..."
	@lsof -ti:5002 | xargs kill -9 2>/dev/null || true
	@echo "Starting API server with hot reload..."
	cargo watch -x run

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
