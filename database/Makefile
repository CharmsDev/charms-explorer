# Charms Explorer Database Makefile

.PHONY: makeDev dev start stop migrate fresh

# Development server
makeDev:
	@echo "Starting database..."
	/usr/local/bin/docker compose up -d

# Alias for makeDev
dev: makeDev

start:
	/usr/local/bin/docker compose up -d

stop:
	/usr/local/bin/docker compose down

migrate:
	cargo run -- migrate

# Fresh start - REMOVED FOR SAFETY
# fresh:
# 	@echo "Command disabled for safety"