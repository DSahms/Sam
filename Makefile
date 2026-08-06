# Sammy top-level task runner. Works in Git Bash on Windows. Each target maps
# to the repeatable commands documented in docs/DEVELOPMENT_RUNBOOK.md.

.PHONY: help install dev build fmt lint test test-rust test-frontend check clean

help:
	@echo "Sammy development targets:"
	@echo "  make install     - install frontend deps + build Rust deps"
	@echo "  make dev         - run the Tauri app in dev mode"
	@echo "  make build       - production build (frontend + Tauri bundle)"
	@echo "  make fmt         - format all code"
	@echo "  make lint        - clippy + eslint + prettier --check + tsc"
	@echo "  make test        - run Rust + frontend tests"
	@echo "  make check       - fmt + lint + test (the CI gate)"

install:
	npm install

dev:
	npm run tauri:dev

build:
	npm run tauri:build

fmt:
	cargo fmt --all
	npm run format

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	npm run typecheck
	npm run lint
	npm run format:check

test: test-rust test-frontend

test-rust:
	cargo test --workspace

test-frontend:
	npm run test

check: lint test

clean:
	cargo clean
	rm -rf dist node_modules
