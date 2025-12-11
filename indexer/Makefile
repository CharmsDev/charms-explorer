# Charms Indexer Makefile

PORT=8002

# Development server - kills any previous instances before starting
.PHONY: dev
dev:
	@echo "🧹 Killing any existing charms-indexer processes..."
	@pkill -9 -f "charms-indexer" 2>/dev/null || true
	@echo "🧹 Killing any process using port $(PORT)..."
	@lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || true
	@sleep 1
	@echo "🚀 Starting indexer (DEBUG mode)..."
	@RUST_LOG=info cargo run --bin charms-indexer

# Build the indexer (release mode)
.PHONY: build
build:
	@echo "🔨 Building indexer (Release)..."
	@cargo build --release

# Clean build artifacts
.PHONY: clean
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
