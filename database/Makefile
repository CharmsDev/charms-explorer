# Charms Explorer Database Makefile

.PHONY: makeDev dev start stop migrate fresh

# Development server
makeDev:
	@echo "Starting database..."
	docker-compose up -d

# Alias for makeDev
dev: makeDev

start:
	docker-compose up -d

stop:
	docker-compose down

migrate:
	cargo run -- migrate

# Fresh start - REMOVED FOR SAFETY
# fresh:
# 	@echo "Command disabled for safety"