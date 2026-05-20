.PHONY: help run resume sample test build check fmt clippy

.DEFAULT_GOAL := help

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-8s\033[0m %s\n", $$1, $$2}'

run: ## Run the dev server (resumes the most recent game)
	cargo run

resume: ## Resume the most recent saved game (alias for run)
	cargo run

sample: ## Run with a fresh sample game loaded
	SAMPLE_GAME=true cargo run

test: ## Run the test suite
	cargo test

build: ## Build the optimized release binary (use for game night)
	cargo build --release

check: fmt clippy test ## Run everything CI checks (fmt, clippy, test)

fmt: ## Check formatting (cargo fmt --check)
	cargo fmt --check

clippy: ## Lint with clippy (-D warnings)
	cargo clippy --all-targets -- -D warnings
