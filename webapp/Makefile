# Makefile for Charms Explorer Webapp

PORT=8001

# Development server with port cleanup
makeDev:
	@echo "Killing any process using port $(PORT)..."
	@lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || true
	@echo "Starting Next.js dev server on port $(PORT)..."
	npm run dev

# Alias for makeDev
dev: makeDev

# Kill any process running on the specified port
kill:
	@echo "Attempting to kill process on port $(PORT)..."
	@-lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || echo "No process found on port $(PORT)."

.PHONY: makeDev dev kill
