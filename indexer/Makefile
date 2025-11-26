# Charms Indexer Makefile

PORT=8002

# Development server with port cleanup
.PHONY: makeDev
makeDev:
	@echo "Killing any process using port $(PORT)..."
	@lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || true
	@echo "🚀 Running indexer..."
	RUST_LOG=info cargo run --bin charms-indexer

# Alias for makeDev
.PHONY: dev
dev: makeDev

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

# Setup development environment
.PHONY: setup
setup:
	@echo "🔧 Setting up development environment..."
	cargo check
