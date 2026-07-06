.PHONY: test test-env test-env-down test-e2e lint format check coverage ci-backend ci-frontend help
.PHONY: dev-up dev-down prod-up prod-down docker-logs docker-health docker-shell
.PHONY: dev-build dev-build-backend dev-build-frontend build-all
.PHONY: prod-build prod-build-backend prod-build-frontend

# VEDO hub RAG Assistant — Makefile

# === Smoke Test ===

smoke: ## Run smoke tests (start services via Docker Compose and verify health)
	@echo "Running smoke tests..."
	@bash scripts/smoke-test.sh --full

# === Docker Development ===

dev-up: ## Start development environment (parallel build)
	docker compose up -d --parallel

dev-down: ## Stop development environment
	docker compose down

dev-logs: ## Follow development logs
	docker compose logs -f

# === Docker Production ===

prod-up: ## Start production environment
	docker compose -f docker-compose.yml -f docker-compose.production.yml up -d

prod-down: ## Stop production environment
	docker compose -f docker-compose.yml -f docker-compose.production.yml down

prod-build: ## Build all production images (parallel)
	docker compose -f docker-compose.yml -f docker-compose.production.yml build --parallel

build-all: ## Build all development images (parallel)
	docker compose build --parallel

dev-build: ## Build all development images (alias for build-all)
	docker compose build --parallel

dev-build-backend: ## Build only backend (development)
	docker compose build backend

dev-build-frontend: ## Build only frontend (development)
	docker compose build frontend

prod-build-backend: ## Build only backend (production)
	docker compose -f docker-compose.yml -f docker-compose.production.yml build backend

prod-build-frontend: ## Build only frontend (production)
	docker compose -f docker-compose.yml -f docker-compose.production.yml build frontend

# === Docker Utilities ===

docker-logs: ## View container logs (usage: make docker-logs ARGS="backend")
	docker compose logs -f $(ARGS)

docker-health: ## Check container health status
	docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}"

docker-health-check: ## Verify all containers are healthy (exit 0 only if all healthy)
	@bash scripts/check-container-health.sh docker-compose.yml docker-compose.override.yml

docker-validate: ## Validate Docker Compose config for common service URL misconfigurations
	@bash scripts/validate-docker-compose.sh



docker-shell: ## Open shell in a container (usage: make docker-shell SVC=backend)
	docker compose exec $(SVC) sh

docker-clean: ## Remove all stopped containers and unused volumes
	docker compose down -v --remove-orphans

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# === Testing ===

test-env: ## Start test environment (docker-compose.test.yml)
	docker compose --env-file .env.test -f docker-compose.test.yml up -d
	@echo "Waiting for all services to be healthy..."
	@sleep 10
	@docker compose --env-file .env.test -f docker-compose.test.yml ps

test-env-down: ## Stop and clean test environment
	docker compose --env-file .env.test -f docker-compose.test.yml down -v

test: ## Run all tests (backend + frontend)
	cd backend && cargo test --lib
	cd backend && cargo test --test integration -- --test-threads=1
	cd frontend && pnpm test

test-e2e: ## Run Playwright e2e inside test_internal network (requires test-env)
	docker compose --env-file .env.test -f docker-compose.test.yml \
		--profile test-runner run --rm frontend-tests

test:keycloak-template: ## Validate keycloak realm template substitution (no Docker needed)
	@bash scripts/validate-keycloak-template.sh

lint: ## Run all linters
	cd backend && cargo clippy -- -D warnings
	cd frontend && pnpm run lint:ci

# === Formatting ===

format: ## Format all code
	cd backend && cargo fmt
	cd frontend && pnpm run format

# === Full check ===

check: validate-migrations format lint test ## Format + lint + test (fail-fast)

# === Migration Validation ===

validate-migrations: ## Validate sqlx migration files (duplicates, gaps, naming)
	@bash scripts/validate-migrations.sh --git

# === Coverage ===

coverage: ## Generate coverage reports
	cd backend && cargo tarpaulin --out Xml --target-dir target/coverage 2>/dev/null || \
		echo "[WARN] tarpaulin not installed"

# === CI targets ===

ci-backend: ## Backend CI (format + lint + test)
	cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib && cargo test --test integration -- --test-threads=1

ci-frontend: ## Frontend CI (lint + format check + test + build)
	cd frontend && pnpm run lint:ci && pnpm run format:check && pnpm run test -- --run && pnpm run build
