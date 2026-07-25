# Synaptic Four — common local verbs (fmt/lint/test).
# Docker stack up/down/destroy will be added when a local compose path exists.

.PHONY: fmt clippy test check deny verify

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

deny:
	cargo deny check licenses
	cargo deny check sources

check: fmt clippy test

verify:
	./scripts/verify.sh
