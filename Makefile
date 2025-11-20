# Makefile for RustCare Engine
# Provides convenient commands for building, testing, and deploying

.PHONY: help build test clean docker-build docker-up docker-down install release

# Default target
help:
	@echo "RustCare Engine - Available Commands:"
	@echo ""
	@echo "  make build          - Build the project in debug mode"
	@echo "  make release        - Build optimized release binary"
	@echo "  make test           - Run all tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make docker-build   - Build Docker images"
	@echo "  make docker-up      - Start Docker containers"
	@echo "  make docker-down    - Stop Docker containers"
	@echo "  make docker-logs    - View Docker logs"
	@echo "  make install        - Install binary to system"
	@echo "  make package        - Create distribution package"
	@echo "  make migrate        - Run database migrations"
	@echo "  make fmt            - Format code"
	@echo "  make lint           - Run linter"
	@echo "  make check          - Run all checks (fmt, lint, test)"

# Build debug
build:
	cargo build

# Build release
release:
	cargo build --release

# Run tests
test:
	cargo test --all

# Clean build artifacts
clean:
	cargo clean
	rm -rf dist/

# Docker commands
docker-build:
	docker-compose build

docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-logs:
	docker-compose logs -f

docker-restart:
	docker-compose restart

# Build release package
package:
	./scripts/build-release.sh

# Install binary
install:
	sudo ./scripts/install.sh

# Database migrations
migrate:
	sqlx migrate run

migrate-revert:
	sqlx migrate revert

# Code formatting
fmt:
	cargo fmt --all

# Linting
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Run all checks
check: fmt lint test
	@echo "All checks passed!"

# Development server
dev:
	cargo run --bin rustcare-server

# Run with watch mode (requires cargo-watch)
watch:
	cargo watch -x "run --bin rustcare-server"

# Generate documentation
docs:
	cargo doc --no-deps --open

# Security audit
audit:
	cargo audit

# Update dependencies
update:
	cargo update

# Full release process
full-release: clean test lint package
	@echo "Release build complete!"
	@echo "Package location: dist/"

