#!/usr/bin/env bash
# Baseline verification — run this FIRST in Cursor / any dev machine, before
# building on top of the current state. Requires rustup (not just system
# cargo/rustc) so the pin in rust-toolchain.toml is actually honoured.
# Also needs libsodium headers (crypt4gh → sodiumoxide → libsodium-sys);
# missing libsodium is warned, not a hard fail — cargo will error later.
#
# Usage: ./scripts/verify.sh
set -euo pipefail

cd "$(dirname "$0")/.."

# Soft check: Crypt4GH pulls libsodium-sys; CI installs libsodium-dev on Ubuntu.
if command -v pkg-config >/dev/null 2>&1; then
  if ! pkg-config --exists libsodium; then
    echo "warning: libsodium not found (pkg-config). Install it first — e.g. brew install libsodium / apt install libsodium-dev"
  fi
else
  echo "warning: pkg-config not found; cannot verify libsodium. Install libsodium if cargo build fails on libsodium-sys."
fi

PINNED_REV="$(grep -v '^#' config/ci/ferrum-revision.txt | head -1 | tr -d '[:space:]')"
CARGO_REV="$(grep -o 'rev = "[a-f0-9]*"' crates/crypto/Cargo.toml | grep -o '[a-f0-9]\{40\}')"

echo "== 0. Sanity: ferrum-core pin consistency =="
if [ "$PINNED_REV" != "$CARGO_REV" ]; then
  echo "MISMATCH: config/ci/ferrum-revision.txt ($PINNED_REV) != crates/crypto/Cargo.toml ($CARGO_REV)"
  exit 1
fi
echo "ok: both pin $PINNED_REV"

echo "== 1. Toolchain =="
if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup not found. Install via https://rustup.rs, then re-run."
  exit 1
fi
rustup show active-toolchain || rustup toolchain install "$(grep channel rust-toolchain.toml | cut -d'"' -f2)"

echo "== 2. fmt =="
cargo fmt --all -- --check

echo "== 3. clippy (deny warnings) =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== 4. test =="
cargo test --workspace --all-targets
cargo test --workspace --doc

echo "== 5. cargo-deny (licenses + sources + bans + advisories) =="
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check licenses
  cargo deny check sources
  cargo deny check bans
  cargo deny check advisories
else
  echo "skip: cargo-deny not installed (cargo install cargo-deny)"
fi

echo "== 6. CLI smoke test =="
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
if SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml; then
  echo "FAIL: non-EU storage region should have been refused"
  exit 1
else
  echo "ok: non-EU storage region correctly refused"
fi

echo "== 7. Reference deployments =="
# Default workspace resolve must NOT pull optional ferrum-storage (Mode A / unfeatured builds).
if cargo tree --workspace --edges normal,build 2>/dev/null | grep -E 'ferrum-storage' >/dev/null; then
  echo "FAIL: ferrum-storage appears in default (no --features) dependency tree"
  exit 1
fi
echo "ok: default workspace tree has no ferrum-storage"

# Mode A — standalone CLI against a fictional EHR/DB (no Ferrum).
./examples/standalone/run.sh
# Mode B — Crypt4GH format interop + AuthClaims smoke (git-pinned ferrum-core).
cargo run -p solum-example-ferrum-companion
echo "ok: both reference deployments passed"

echo "== 7b. Ferrum-storage backend (feature-gated; rustc >= 1.94.1) =="
if rustc +1.94.1 -vV >/dev/null 2>&1; then
  RUSTUP_TOOLCHAIN=1.94.1 cargo test -p solum-core --features ferrum-storage-backend --lib
  RUSTUP_TOOLCHAIN=1.94.1 cargo run -p solum-example-ferrum-companion --features storage-backend
  echo "ok: ferrum-storage-backend feature path passed"
else
  echo "SKIP: rustc 1.94.1 not installed (ferrum-storage pulls aws-sdk which needs it)"
fi

echo "== 7c. AWS KMS feature (mocked; rustc >= 1.94.1) =="
if rustc +1.94.1 -vV >/dev/null 2>&1; then
  RUSTUP_TOOLCHAIN=1.94.1 cargo test -p solum-crypto --features aws-kms --test aws_kms
  RUSTUP_TOOLCHAIN=1.94.1 cargo check -p solum-core --features aws-kms
  RUSTUP_TOOLCHAIN=1.94.1 cargo check -p solum-sidecar --features aws-kms
  echo "ok: aws-kms feature path passed"
else
  echo "SKIP: rustc 1.94.1 not installed (aws-kms feature requires it for AWS SDK)"
fi

echo "== 8. Compliance worked example (Deny paths + retained artifacts) =="
./examples/compliance-worked-example/run.sh
echo "ok: compliance worked example passed"

echo
echo "All baseline checks passed."
