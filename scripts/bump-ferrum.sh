#!/usr/bin/env bash
# Bump the Ferrum git pin (ferrum-core) — same pattern as Ferrum Lab Kit.
# Updates: crates/crypto/Cargo.toml, crates/core/Cargo.toml, examples/ferrum-companion/Cargo.toml,
#          crates/crypto/src/lib.rs (FERRUM_GIT_REV), config/ci/ferrum-revision.txt
#
# Usage:
#   ./scripts/bump-ferrum.sh              # use origin/main tip
#   ./scripts/bump-ferrum.sh <40-char-sha> # pin exact commit
#   ./scripts/bump-ferrum.sh --dry-run    # show SHA only, do not write files
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FERRUM_REMOTE="${FERRUM_REMOTE:-https://github.com/SynapticFour/Ferrum.git}"

usage() {
  cat <<'EOF'
Usage: ./scripts/bump-ferrum.sh [--dry-run] [<full-40-hex-sha>]

  --dry-run   Print resolved revision and exit without editing files.
  <sha>       Pin this commit (40 lowercase hex chars). Otherwise uses refs/heads/main.

Environment:
  FERRUM_REMOTE   Git URL (default: https://github.com/SynapticFour/Ferrum.git)

After bumping:
  cargo update -p ferrum-core
  ./scripts/verify.sh
EOF
}

DRY_RUN=0
SHA_ARG=""

for a in "$@"; do
  case "$a" in
    -h | --help)
      usage
      exit 0
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    *)
      if [[ -n "$SHA_ARG" ]]; then
        echo "error: unexpected extra argument: $a" >&2
        usage >&2
        exit 1
      fi
      SHA_ARG="$a"
      ;;
  esac
done

resolve_sha() {
  if [[ -n "$SHA_ARG" ]]; then
    echo "$SHA_ARG"
    return
  fi
  git ls-remote "$FERRUM_REMOTE" refs/heads/main | awk '{ print $1; exit }'
}

FERRUM_REV="$(resolve_sha)"

if [[ -z "$FERRUM_REV" ]]; then
  echo "error: could not resolve Ferrum revision (git ls-remote failed?)" >&2
  exit 1
fi

if ! [[ "$FERRUM_REV" =~ ^[0-9a-f]{40}$ ]]; then
  echo "error: expected full 40-char lowercase hex SHA, got: $FERRUM_REV" >&2
  exit 1
fi

echo "Ferrum revision: $FERRUM_REV"

if [[ "$DRY_RUN" -eq 1 ]]; then
  exit 0
fi

perl -i -pe "s/rev = \"[0-9a-f]{40}\"/rev = \"$FERRUM_REV\"/" \
  "$ROOT/crates/crypto/Cargo.toml" \
  "$ROOT/crates/core/Cargo.toml" \
  "$ROOT/examples/ferrum-companion/Cargo.toml"

perl -i -pe "s/pub const FERRUM_GIT_REV: &str = \"[0-9a-f]{40}\"/pub const FERRUM_GIT_REV: &str = \"$FERRUM_REV\"/" \
  "$ROOT/crates/crypto/src/lib.rs"

TMP="$(mktemp)"
awk -v sha="$FERRUM_REV" '
  /^[0-9a-f]{40}$/ { print sha; replaced = 1; next }
  { print }
  END { if (!replaced) print sha }
' "$ROOT/config/ci/ferrum-revision.txt" >"$TMP"
mv "$TMP" "$ROOT/config/ci/ferrum-revision.txt"

echo "Updated:"
echo "  - crates/crypto/Cargo.toml"
echo "  - crates/core/Cargo.toml (ferrum-storage optional pin)"
echo "  - examples/ferrum-companion/Cargo.toml"
echo "  - crates/crypto/src/lib.rs (FERRUM_GIT_REV)"
echo "  - config/ci/ferrum-revision.txt"
echo ""
echo "Next:"
echo "  cargo update -p ferrum-core"
echo "  ./scripts/verify.sh"
