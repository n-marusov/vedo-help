.PHONY: test test-env test-env-down test:e2e lint format check coverage ci-backend ci-embedding ci-frontend help
.PHONY: dev-up dev-down prod-up prod-down docker-logs docker-health docker-shell

# VEDO hub RAG Assistant — Makefile

# === Smoke Test ===

smoke: ## Run smoke tests (start services via Docker Compose and verify health)
	@echo "Running smoke tests..."
	@bash scripts/smoke-test.sh --full

# === Docker Development ===

dev-up: ## Start development environment
	docker compose up -d

dev-down: ## Stop development environment
	docker compose down

dev-logs: ## Follow development logs
	docker compose logs -f

# === Docker Production ===

prod-up: ## Start production environment
	docker compose -f docker-compose.yml -f docker-compose.production.yml up -d

prod-down: ## Stop production environment
	docker compose -f docker-compose.yml -f docker-compose.production.yml down

prod-build: ## Build production images
	docker compose -f docker-compose.yml -f docker-compose.production.yml build

# === Docker Utilities ===

docker-logs: ## View container logs (usage: make docker-logs ARGS="backend")
	docker compose logs -f $(ARGS)

docker-health: ## Check container health status
	docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}"

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

test: ## Run all tests (backend + frontend + embedding)
	cd backend && cargo test --lib
	cd frontend && npm test
	cd embedding && pytest tests/ -v

test:e2e: ## Run Playwright e2e inside test_internal network (requires test-env)
	docker compose --env-file .env.test -f docker-compose.test.yml \
		--profile test-runner run --rm frontend-tests

lint: ## Run all linters
	cd backend && cargo clippy -- -D warnings
	cd frontend && npm run lint:ci
	cd embedding && ruff check src/

# === Formatting ===

format: ## Format all code
	cd backend && cargo fmt
	cd frontend && npm run format
	cd embedding && ruff format src/

# === Full check ===

check: format lint test ## Format + lint + test (fail-fast)

# === Coverage ===

coverage: ## Generate coverage reports
	cd backend && cargo tarpaulin --out Xml --target-dir target/coverage 2>/dev/null || \
		echo "[WARN] tarpaulin not installed"
	cd embedding && pytest tests/ -v --cov=src --cov-report=xml --cov-report=term

# === CI targets ===

ci-backend: ## Backend CI (format + lint + test)
	cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib

ci-embedding: ## Embedding CI (format + lint + test)
	cd embedding && ruff format src/ --check && ruff check src/ && pytest tests/ -v --cov=src --cov-report=term

ci-frontend: ## Frontend CI (lint + format check + test + build)
	cd frontend && npm run lint:ci && npm run format:check && npm run test -- --run && npm run build

smoke-dns: ## Run DNS smoke test (check embedding DNS resolution independent of host VPN)
	@echo "Running DNS smoke test..."
	@bash scripts/smoke-test-dns.sh
