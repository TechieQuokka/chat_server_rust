# ============================================
# Chat Server v3 - Makefile
# ============================================
# Common commands for development workflow

.PHONY: help build run dev test lint fmt clean docker-up docker-down db-migrate db-reset docs

# Default target
.DEFAULT_GOAL := help

# ============================================
# Variables
# ============================================
CARGO := cargo
DOCKER_COMPOSE := docker-compose
DATABASE_URL ?= postgres://chat_user:chat_password@localhost:5432/chat_db

# ============================================
# Help
# ============================================
help: ## Show this help message
	@echo "Chat Server v3 - Development Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ============================================
# Build & Run
# ============================================
build: ## Build the project in release mode
	$(CARGO) build --release

build-dev: ## Build the project in debug mode
	$(CARGO) build

run: ## Run the server in release mode
	$(CARGO) run --release

dev: ## Run the server in development mode with auto-reload
	$(CARGO) watch -x 'run' | bunyan

dev-debug: ## Run with debug logging
	RUST_LOG=debug,sqlx=warn,tower_http=trace $(CARGO) watch -x 'run'

# ============================================
# Testing
# ============================================
test: ## Run all tests
	$(CARGO) nextest run

test-unit: ## Run unit tests only
	$(CARGO) nextest run --lib

test-integration: ## Run integration tests only
	$(CARGO) nextest run --test '*'

test-coverage: ## Run tests with coverage report
	$(CARGO) tarpaulin --out html --output-dir target/coverage

test-watch: ## Run tests in watch mode
	$(CARGO) watch -x 'nextest run'

bench: ## Run benchmarks
	$(CARGO) bench

# ============================================
# Code Quality
# ============================================
lint: ## Run clippy linter
	$(CARGO) clippy --all-targets --all-features -- -D warnings

lint-fix: ## Run clippy and apply fixes
	$(CARGO) clippy --all-targets --all-features --fix --allow-dirty

fmt: ## Format code
	$(CARGO) fmt

fmt-check: ## Check code formatting
	$(CARGO) fmt -- --check

audit: ## Run security audit
	$(CARGO) audit

outdated: ## Check for outdated dependencies
	$(CARGO) outdated

check: fmt-check lint test ## Run all checks (format, lint, test)

# ============================================
# Docker
# ============================================
docker-up: ## Start Docker containers
	$(DOCKER_COMPOSE) up -d

docker-down: ## Stop Docker containers
	$(DOCKER_COMPOSE) down

docker-logs: ## View Docker container logs
	$(DOCKER_COMPOSE) logs -f

docker-ps: ## List running containers
	$(DOCKER_COMPOSE) ps

docker-reset: ## Reset Docker containers and volumes
	$(DOCKER_COMPOSE) down -v
	$(DOCKER_COMPOSE) up -d

docker-build: ## Build Docker image
	docker build -t chat-server:latest .

# ============================================
# Database
# ============================================
db-migrate: ## Run database migrations
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

db-migrate-create: ## Create a new migration (usage: make db-migrate-create name=migration_name)
	DATABASE_URL=$(DATABASE_URL) sqlx migrate add $(name)

db-migrate-revert: ## Revert last migration
	DATABASE_URL=$(DATABASE_URL) sqlx migrate revert

db-reset: ## Reset database (drop and recreate)
	DATABASE_URL=$(DATABASE_URL) sqlx database drop -y || true
	DATABASE_URL=$(DATABASE_URL) sqlx database create
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

db-seed: ## Seed database with test data
	psql $(DATABASE_URL) -f scripts/seed-data.sql

db-prepare: ## Prepare SQLx offline cache
	$(CARGO) sqlx prepare --workspace

db-shell: ## Open PostgreSQL shell
	psql $(DATABASE_URL)

redis-shell: ## Open Redis CLI
	redis-cli

# ============================================
# Documentation
# ============================================
docs: ## Generate and open documentation
	$(CARGO) doc --no-deps --open

docs-build: ## Generate documentation
	$(CARGO) doc --no-deps

# ============================================
# Load Testing
# ============================================
load-test: ## Run k6 load test
	k6 run tests/k6/load-test.js

load-test-smoke: ## Run smoke test only
	k6 run --env SCENARIO=smoke tests/k6/load-test.js

# ============================================
# Setup
# ============================================
setup: ## Initial project setup
	@echo "Installing required tools..."
	cargo install sqlx-cli --no-default-features --features postgres,rustls
	cargo install cargo-watch cargo-nextest cargo-audit cargo-outdated cargo-tarpaulin
	@echo ""
	@echo "Starting Docker containers..."
	$(DOCKER_COMPOSE) up -d
	@echo ""
	@echo "Waiting for database..."
	@sleep 5
	@echo ""
	@echo "Running migrations..."
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run || true
	@echo ""
	@echo "Setup complete! Run 'make dev' to start the server."

setup-hooks: ## Setup git hooks
	git config core.hooksPath .githooks
	chmod +x .githooks/*
	@echo "Git hooks configured!"

# ============================================
# Cleanup
# ============================================
clean: ## Clean build artifacts
	$(CARGO) clean

clean-all: clean docker-down ## Clean everything including Docker
	rm -rf target/
	rm -rf .sqlx/

# ============================================
# Release
# ============================================
release-patch: ## Create a patch release
	$(CARGO) release patch --execute

release-minor: ## Create a minor release
	$(CARGO) release minor --execute

release-major: ## Create a major release
	$(CARGO) release major --execute
