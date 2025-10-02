# Simple shortcut makefile for checking and formatting the project
# Client/ and Server/ have both delegation files, making it easier to execute this is in both development scenarios
.PHONY: help fmt-client fmt-server fmt check-client check-server check

help:
	@echo "Available commands:"
	@echo "  make fmt           - Format all code"
	@echo "  make fmt-client    - Format Python client"
	@echo "  make fmt-server    - Format Rust server"
	@echo "  make check         - Run all checks"
	@echo "  make check-client  - Check Python client"
	@echo "  make check-server  - Check Rust server"

# Format Python client
fmt-client:
	@echo "Formatting Python client..."
	@cd client && black . && ruff check --fix .

# Format Rust server
fmt-server:
	@echo "Formatting Rust server..."
	@cd server && cargo fmt

# Format everything
fmt: fmt-client fmt-server

# Check Python client
check-client:
	@echo "Checking Python client..."
	@cd client && black --check . && ruff check .

# Check Rust server
check-server:
	@echo "Checking Rust server..."
	@cd server && cargo fmt -- --check && cargo clippy -- -D warnings

# Check everything
check: check-client check-server