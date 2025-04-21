# Makefile for sign-test project

PORT=7483

# Kill process on specified port and start the dev server
run: kill dev

# Start the development server
dev:
	@echo "Starting Next.js dev server on port $(PORT)..."
	npm run dev

# Kill any process running on the specified port
kill:
	@echo "Attempting to kill process on port $(PORT)..."
	@-lsof -ti:$(PORT) | xargs kill -9 2>/dev/null || echo "No process found on port $(PORT)."

.PHONY: run dev kill
