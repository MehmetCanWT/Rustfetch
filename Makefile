.PHONY: test test-unit test-cli check fmt lint build release

check:
	cargo check --workspace --all-targets --message-format=short

test:
	./scripts/test.sh

test-unit:
	cargo test --workspace --locked

test-cli:
	@tmp="$$(mktemp -d)"; trap 'rm -rf "$$tmp"' EXIT; ./scripts/test-cli.sh "$$tmp"

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings

build:
	cargo build --workspace --locked

release:
	cargo build --workspace --release --locked
