.PHONY: start stop restart fresh help

# Default target
all: start

# Start the database container
start:
	docker-compose up -d
	@echo "Database started on port 5432"

# Stop the database container
stop:
	docker-compose down

# Restart the database container
restart: stop start

# Fresh start - remove all volumes and restart
fresh:
	docker-compose down -v
	docker volume prune -f
	docker-compose up -d
	@echo "Database started fresh on port 5432"

# Help
help:
	@echo "Available targets:"
	@echo "  start        - Start the database container"
	@echo "  stop         - Stop the database container"
	@echo "  restart      - Restart the database container"
	@echo "  fresh        - Remove all volumes and start fresh"
	@echo "  help         - Show this help message"
