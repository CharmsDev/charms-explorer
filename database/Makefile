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

# Fresh start - DANGER: removes charms database volume and restarts
fresh:
	docker-compose down -v
	docker volume prune -f
	docker-compose up -d
	@echo "Database started fresh"