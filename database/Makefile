.PHONY: start stop migrate

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
	@echo "Database started fresh on port 5432"