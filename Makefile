.PHONY: run test sample build

run:
	cd backend && cargo run

test:
	cd backend && cargo test

sample:
	cd backend && SAMPLE_GAME=true cargo run

build:
	cd frontend && npm run build
	rm -rf backend/static/app
	cp -r frontend/dist backend/static/app
