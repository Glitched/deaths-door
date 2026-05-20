.PHONY: help run resume test sample build

.DEFAULT_GOAL := help

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-8s\033[0m %s\n", $$1, $$2}'

run: ## Run the backend (resumes the most recent game)
	cd backend && cargo run

resume: ## Resume the most recent saved game (alias for run)
	cd backend && cargo run

sample: ## Run the backend with a fresh sample game
	cd backend && SAMPLE_GAME=true cargo run

test: ## Run backend tests
	cd backend && cargo test

build: ## Build the frontend into backend/static/app
	cd frontend && npm run build
	rm -rf backend/static/app
	cp -r frontend/dist backend/static/app
