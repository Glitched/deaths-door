.PHONY: run sample test build check fmt clippy

run:
	cargo run

sample:
	SAMPLE_GAME=true cargo run

test:
	cargo test

# Optimized binary (use for game night — faster, better DMX timing).
build:
	cargo build --release

# Everything CI checks.
check: fmt clippy test

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings
