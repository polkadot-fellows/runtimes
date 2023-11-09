default: ci

ci: fmt check-no-std clippy test bench-test

test:
	cargo test --all --all-features

bench-test:
	cargo bench -- --test

clippy:
	cargo clippy  --all --all-features --all-targets

fmt:
	cargo fmt --all -- --check

check-no-std:
	cargo check --all --no-default-features
