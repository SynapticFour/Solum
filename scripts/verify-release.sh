#!/usr/bin/env bash
# Local gate before a v* tag.
# scripts/verify.sh is the fmt, clippy, test, cargo-deny, and feature-path
# gate. CodeQL stays workflow_dispatch (see docs/CI.md).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/hooks/macos-sdk.sh"
use_linkable_macos_sdk
exec "$ROOT/scripts/verify.sh"
