# Charms Indexer Makefile

PORT=8002
PID_FILE=.indexer.pid

# Run all tests (unit + integration). Integration tests need a Docker daemon
# running because they start ephemeral Postgres containers via testcontainers.
.PHONY: test
test:
	@cargo test --all-targets

# Quick unit-only loop (no containers).
.PHONY: test-unit
test-unit:
	@cargo test --lib

# Strict lint pass (advisory until warnings are driven to zero).
.PHONY: lint
lint:
	@cargo clippy --all-targets

# Coverage report via cargo-llvm-cov (install with `cargo install cargo-llvm-cov`).
# Writes target/llvm-cov/html/index.html.
.PHONY: coverage
coverage:
	@cargo llvm-cov --all-targets --html --output-dir target/llvm-cov

# Development server - kills any previous instances before starting
.PHONY: dev
dev: stop
	@echo "🚀 Starting indexer (DEBUG mode)..."
	@RUST_LOG=info cargo run --bin charms-indexer & echo $$! > $(PID_FILE)
	@echo "✅ Indexer started with PID $$(cat $(PID_FILE))"

# Stop the indexer
.PHONY: stop
stop:
	@echo "🛑 Stopping indexer..."
	@if [ -f $(PID_FILE) ]; then \
		PID=$$(cat $(PID_FILE)); \
		if kill -0 $$PID 2>/dev/null; then \
			echo "   Killing process $$PID from PID file..."; \
			kill -9 $$PID 2>/dev/null || true; \
		fi; \
		rm -f $(PID_FILE); \
	fi
	@echo "   Killing any charms-indexer processes..."
	@pkill -9 -f "target/debug/charms-indexer" 2>/dev/null || true
	@pkill -9 -f "target/release/charms-indexer" 2>/dev/null || true
	@echo "   Killing any process using port $(PORT)..."
	@lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || true
	@sleep 1
	@echo "✅ Indexer stopped"

# Run in foreground (blocking) - useful for seeing logs directly
.PHONY: run
run: stop
	@echo "🚀 Starting indexer in foreground..."
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

# Show status
.PHONY: status
status:
	@echo "📊 Indexer Status:"
	@if [ -f $(PID_FILE) ]; then \
		PID=$$(cat $(PID_FILE)); \
		if kill -0 $$PID 2>/dev/null; then \
			echo "   ✅ Running (PID: $$PID)"; \
		else \
			echo "   ❌ Not running (stale PID file)"; \
		fi; \
	else \
		echo "   ❌ Not running (no PID file)"; \
	fi
	@echo "   Processes matching 'charms-indexer':"
	@pgrep -fl "charms-indexer" || echo "   (none)"
	@echo "   Port $(PORT) usage:"
	@lsof -i:$(PORT) 2>/dev/null || echo "   (not in use)"
