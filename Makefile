# Synaptic Four — common local verbs (fmt/lint/test).
# No generic product Docker image in this repo. Track B evaluation image:
# deploy/h3-ehrbase/Dockerfile.sidecar (dev-local + SOLUM_ALLOW_PLAINTEXT_HTTP;
# consumed by Solum-Demo compose). Production: reverse-proxy TLS in front of 127.0.0.1.

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
