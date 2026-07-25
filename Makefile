.PHONY: test test-env test-env-down test-e2e lint format check coverage ci-backend ci-frontend help
.PHONY: dev-up dev-down prod-up prod-down docker-logs docker-health docker-shell
.PHONY: dev-build dev-build-backend dev-build-frontend build-all
.PHONY: prod-build prod-build-backend prod-build-frontend
.PHONY: backup restore backup-schedule
.PHONY: smoke prod-smoke docker-login docker-push deploy
.PHONY: load-test load-test-full load-test-compare

# VEDO hub RAG Assistant — Makefile

# Compose files base path
COMPOSE_DIR := deploy/docker

# Base compose command: pass --env-file .env explicitly so Docker Compose
# finds the project-root .env (compose files are now under deploy/docker/).
COMPOSE_BASE := docker compose --env-file .env -f $(COMPOSE_DIR)/compose.yml

# Dev compose: includes override for Vite dev server with hot-reload on port 5173.
COMPOSE_DEV := $(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.override.yml

# Default container registry namespace
REGISTRY_NS ?= ghcr.io/vedo
# Default version tag for docker-push
VERSION ?= $(shell git rev-parse --short HEAD)

# === Smoke Test ===

smoke: ## Run smoke tests (start services via Docker Compose and verify health)
	@echo "Running smoke tests..."
	bash scripts/ops/smoke-test.sh --full

prod-smoke: ## Run smoke tests with production compose profile
	@echo "Running production smoke tests..."
	@bash scripts/ops/smoke-test.sh --production --full

# === Docker Registry ===

docker-login: ## Log in to GitHub Container Registry
	@echo "Logging in to ghcr.io..."
	@echo "$${GITHUB_TOKEN}" | docker login ghcr.io -u "$${GITHUB_USER:-$(shell whoami)}" --password-stdin

docker-push: ## Build & push images to registry
	@echo "Building and pushing images..."
	$(COMPOSE_BASE) build --parallel
	@for svc in backend frontend; do \
		img=$$($(COMPOSE_BASE) images -q $$svc); \
		if [ -z "$$img" ]; then \
			echo "ERROR: No image found for $$svc — was the build successful?"; \
			exit 1; \
		fi; \
		docker tag "$$img" $(REGISTRY_NS)/$$svc:$(VERSION); \
		docker push $(REGISTRY_NS)/$$svc:$(VERSION); \
	done

deploy: ## Deploy to production VPS (run smoke tests before deploy)
	@echo "Running pre-deploy smoke tests..."
	@bash scripts/ops/smoke-test.sh --production --quick
	@echo ""
	@echo "To deploy via CI, push to main: git push origin main"
	@echo "To deploy manually:"
	@echo "  ssh <host> 'cd <project-dir> && docker compose --env-file .env -f deploy/docker/compose.yml -f deploy/docker/compose.production.yml pull backend frontend'"
	@echo "  ssh <host> 'cd <project-dir> && docker compose --env-file .env -f deploy/docker/compose.yml -f deploy/docker/compose.production.yml up -d --no-deps backend frontend'"

# === Docker Development ===

dev-up: ## Start development environment
	$(COMPOSE_DEV) up -d --parallel

dev-down: ## Stop development environment
	$(COMPOSE_DEV) down

dev-logs: ## Follow development logs
	$(COMPOSE_DEV) logs -f

# === Docker Production ===

prod-up: ## Start production environment
	$(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.production.yml up -d

prod-down: ## Stop production environment
	$(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.production.yml down

prod-build: ## Build all production images (parallel)
	$(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.production.yml build --parallel

build-all: ## Build all development images (parallel)
	$(COMPOSE_DEV) build --parallel

dev-build: ## Build all development images (alias for build-all)
	$(COMPOSE_DEV) build --parallel

dev-build-backend: ## Build only backend (development)
	$(COMPOSE_DEV) build backend

dev-build-frontend: ## Build only frontend (development)
	$(COMPOSE_DEV) build frontend

prod-build-backend: ## Build only backend (production)
	$(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.production.yml build backend

prod-build-frontend: ## Build only frontend (production)
	$(COMPOSE_BASE) -f $(COMPOSE_DIR)/compose.production.yml build frontend

# === Docker Utilities ===

docker-logs: ## View container logs (usage: make docker-logs ARGS="backend")
	$(COMPOSE_BASE) logs -f $(ARGS)

docker-health: ## Check container health status
	$(COMPOSE_BASE) ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}"

docker-health-check: ## Verify all containers are healthy
	@bash scripts/ops/check-container-health.sh $(COMPOSE_DIR)/compose.yml $(COMPOSE_DIR)/compose.override.yml

docker-validate: ## Validate Docker Compose config
	@bash scripts/ci/validate-docker-compose.sh

docker-shell: ## Open shell in a container (usage: make docker-shell SVC=backend)
	$(COMPOSE_DEV) exec $(SVC) sh

docker-clean: ## Remove all stopped containers and unused volumes
	$(COMPOSE_DEV) down -v --remove-orphans

# === Backup & Restore ===

backup: ## Run backup script (usage: make backup ARGS="--prod")
	bash scripts/ops/backup.sh $(ARGS)

restore: ## Run restore script (usage: make restore ARGS="<vedo_dump> <keycloak_dump> [chroma_archive]")
	bash scripts/ops/restore.sh $(ARGS)

backup-schedule: ## Print instructions for scheduling automated backups
	@echo "To schedule daily backups, add a cron job or systemd timer:"
	@echo ""
	@echo "  # ── Cron (daily at 2am) ──────────────────────────────────"
	@echo "  0 2 * * * cd $(PWD) && bash scripts/ops/backup.sh --prod >> /var/log/vedo-backup.log 2>&1"
	@echo ""
	@echo "  # ── systemd timer (daily at 2am) ─────────────────────────"
	@echo "  # /etc/systemd/system/vedo-backup.service"
	@echo "  [Unit]"
	@echo "  Description=VEDO hub daily backup"
	@echo "  [Service]"
	@echo "  Type=oneshot"
	@echo "  WorkingDirectory=$(PWD)"
	@echo "  ExecStart=/usr/bin/bash scripts/ops/backup.sh --prod"
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
	k6 run tests/load/smoke-test.js
	@echo "Smoke test passed. Running load test..."
	k6 run tests/load/load-test.js

load-test-full: ## Run all 4 load test scenarios
	@echo "Running full load test suite..."
	k6 run tests/load/smoke-test.js
	@echo ""
	@echo "=== Load Test ==="
	k6 run tests/load/load-test.js
	@echo ""
	@echo "=== Stress Test ==="
	k6 run tests/load/stress-test.js
	@echo ""
	@echo "=== Soak Test (30 min) ==="
	k6 run tests/load/soak-test.js

load-test-compare: ## Compare current results against baseline
	@echo "Load test comparison (run a load test first to generate baseline)"
	@echo "Usage: k6 run --out json=tests/load/results.json tests/load/load-test.js"
	@echo ""
	@echo "To compare two result files:"
	@echo "  k6 run --out json=tests/load/new.json tests/load/load-test.js"
	@echo "  # Then compare manually or with a diff tool"

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# === Testing ===

test-env: ## Start test environment
	docker compose --env-file .env.test -f $(COMPOSE_DIR)/compose.test.yml up -d
	@echo "Waiting for all services to be healthy..."
	@sleep 10
	@docker compose --env-file .env.test -f $(COMPOSE_DIR)/compose.test.yml ps

test-env-down: ## Stop and clean test environment
	docker compose --env-file .env.test -f $(COMPOSE_DIR)/compose.test.yml down -v

test: ## Run all tests (backend + frontend)
	cd backend && cargo test --lib
	cd backend && cargo test --test integration -- --test-threads=1
	cd frontend && pnpm test

test-e2e: ## Run Playwright e2e inside test_internal network (requires test-env)
	docker compose --env-file .env.test -f $(COMPOSE_DIR)/compose.test.yml \
		--profile test-runner run --rm frontend-tests

test:keycloak-template: ## Validate keycloak realm template substitution
	@bash scripts/ci/validate-keycloak-template.sh

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
	@bash scripts/ci/validate-migrations.sh --git

# === Coverage ===

coverage: ## Generate coverage reports
	cd backend && cargo tarpaulin --out Xml --target-dir target/coverage 2>/dev/null || \
		echo "[WARN] tarpaulin not installed"

# === CI targets ===

ci-backend: ## Backend CI (format + lint + test)
	cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib && cargo test --test integration -- --test-threads=1

ci-frontend: ## Frontend CI (lint + format check + test + build)
	cd frontend && pnpm run lint:ci && pnpm run format:check && pnpm run test -- --run && pnpm run build
