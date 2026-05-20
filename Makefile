.PHONY: run test sample build

run:
	cd backend-rust && cargo run

test:
	cd backend-rust && cargo test

sample:
	cd backend-rust && SAMPLE_GAME=true cargo run

build:
	cd frontend && npm run build
	rm -rf backend-rust/static/app
	cp -r frontend/dist backend-rust/static/app
