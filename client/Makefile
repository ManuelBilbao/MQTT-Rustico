build:
	cargo build

run:
	cargo run 127.0.0.1 1883

clippy:
	cargo clippy -- -D warnings

format:
	cargo fmt --all

git: clippy format
	cargo fmt --all -- --check
