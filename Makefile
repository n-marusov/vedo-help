.PHONY: test test-env test-env-down test-e2e lint format check coverage ci-backend ci-frontend help
.PHONY: dev-up dev-down prod-up prod-down docker-logs docker-health docker-shell
.PHONY: dev-build dev-build-backend dev-build-frontend build-all
.PHONY: prod-build prod-build-backend prod-build-frontend
.PHONY: backup restore backup-schedule
.PHONY: smoke prod-smoke docker-login docker-push deploy
.PHONY: load-test load-test-full load-test-compare

# VEDO hub RAG Assistant — Makefile

# Default container registry namespace
REGISTRY_NS ?= ghcr.io/vedo
# Default version tag for docker-push
VERSION ?= $(shell git rev-parse --short HEAD)

# === Smoke Test ===

smoke: ## Run smoke tests (start services via Docker Compose and verify health)
	@echo "Running smoke tests..."
	@bash scripts/smoke-test.sh --full

prod-smoke: ## Run smoke tests with production compose profile
	@echo "Running production smoke tests..."
	@bash scripts/smoke-test.sh --production --full

# === Docker Registry ===

docker-login: ## Log in to GitHub Container Registry (usage: make docker-login GITHUB_TOKEN=ghp_...)
	@echo "Logging in to ghcr.io..."
	@echo "$${GITHUB_TOKEN}" | docker login ghcr.io -u "$${GITHUB_USER:-$(shell whoami)}" --password-stdin

docker-push: ## Build & push images to registry (usage: REGISTRY_NS=ghcr.io/my-org make docker-push VERSION=v1.0.0)
	@echo "Building and pushing images..."
	docker compose build --parallel
	@for svc in backend frontend; do \
		img=$$(docker compose images -q $$svc); \
		if [ -z "$$img" ]; then \
			echo "ERROR: No image found for $$svc — was the build successful?"; \
			exit 1; \
		fi; \
		docker tag "$$img" $(REGISTRY_NS)/$$svc:$(VERSION); \
		docker push $(REGISTRY_NS)/$$svc:$(VERSION); \
	done

deploy: ## Deploy to production VPS (run smoke tests before deploy)
	@echo "Running pre-deploy smoke tests..."
	@bash scripts/smoke-test.sh --production --quick
	@echo ""
	@echo "To deploy via CI, push to main: git push origin main"
	@echo "To deploy manually:"
	@echo "  ssh <host> 'cd <project-dir> && docker compose pull && docker compose up -d --no-deps backend frontend'"

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

# === Backup & Restore ===

backup: ## Run backup script (usage: make backup ARGS="--prod")
	bash scripts/backup.sh $(ARGS)

restore: ## Run restore script (usage: make restore ARGS="<vedo_dump> <keycloak_dump> [chroma_archive]")
	bash scripts/restore.sh $(ARGS)

backup-schedule: ## Print instructions for scheduling automated backups
	@echo "To schedule daily backups, add a cron job or systemd timer:"
	@echo ""
	@echo "  # ── Cron (daily at 2am) ──────────────────────────────────"
	@echo "  0 2 * * * cd $(PWD) && bash scripts/backup.sh --prod >> /var/log/vedo-backup.log 2>&1"
	@echo ""
	@echo "  # ── systemd timer (daily at 2am) ─────────────────────────"
	@echo "  # /etc/systemd/system/vedo-backup.service"
	@echo "  [Unit]"
	@echo "  Description=VEDO hub daily backup"
	@echo "  [Service]"
	@echo "  Type=oneshot"
	@echo "  WorkingDirectory=$(PWD)"
	@echo "  ExecStart=/usr/bin/bash scripts/backup.sh --prod"
	@echo "  StandardOutput=append:/var/log/vedo-backup.log"
	@echo "  StandardError=append:/var/log/vedo-backup.log"
	@echo ""
	@echo "  # /etc/systemd/system/vedo-backup.timer"
	@echo "  [Unit]"
	@echo "  Description=Daily VEDO hub backup timer"
	@echo "  [Timer]"
	@echo "  OnCalendar=daily"
	@echo "  Persistent=true"
	@echo "  [Install]"
	@echo "  WantedBy=timers.target"
	@echo ""
	@echo "  # Enable:"
	@echo "  sudo systemctl daemon-reload"
	@echo "  sudo systemctl enable --now vedo-backup.timer"
	@echo ""
	@echo "  # Verify:"
	@echo "  sudo systemctl list-timers --all | grep vedo"

# === Load Testing ===

load-test: ## Run smoke + load test scenarios
	@echo "Running load tests (smoke + load)..."
	k6 run load-tests/smoke-test.js
	@echo "Smoke test passed. Running load test..."
	k6 run load-tests/load-test.js

load-test-full: ## Run all 4 load test scenarios (smoke, load, stress, soak)
	@echo "Running full load test suite..."
	k6 run load-tests/smoke-test.js
	@echo ""
	@echo "=== Load Test ==="
	k6 run load-tests/load-test.js
	@echo ""
	@echo "=== Stress Test ==="
	k6 run load-tests/stress-test.js
	@echo ""
	@echo "=== Soak Test (30 min) ==="
	k6 run load-tests/soak-test.js

load-test-compare: ## Compare current results against baseline
	@echo "Load test comparison (run a load test first to generate baseline)"
	@echo "Usage: k6 run --out json=load-tests/results.json load-tests/load-test.js"
	@echo ""
	@echo "To compare two result files:"
	@echo "  k6 run --out json=load-tests/new.json load-tests/load-test.js"
	@echo "  # Then compare manually or with a diff tool"

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
