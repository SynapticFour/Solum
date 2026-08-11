#!/usr/bin/env bash
# Offline Prefer / Cut-over rehearsal helper for H3.2 migration stages.
# Does NOT talk to a live partner EHR or EHRbase — validates local tooling
# and prints the operator checklist stages that still need a live site.
#
# Usage: ./scripts/migration-rehearsal-dry-run.sh
# See: docs/MIGRATION-CUTOVER-CHECKLIST.md
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT_DIR="${SOLUM_MIGRATION_REHEARSAL_OUT:-$ROOT/examples/compliance-worked-example/artifacts/migration-rehearsal}"
mkdir -p "$OUT_DIR"
# gitignored under examples/compliance-worked-example/artifacts/migration-rehearsal/

echo "======== Migration Prefer/Cut-over dry rehearsal ========"
echo "checklist: docs/MIGRATION-CUTOVER-CHECKLIST.md"
echo "out:       $OUT_DIR"
echo

SAMPLE_BUNDLE="$OUT_DIR/sample-bundle.json"
INVENTORY="$OUT_DIR/inventory.jsonl"
DEAD_LETTER="$OUT_DIR/dual-write-dead-letter.jsonl"
PAYLOAD="$OUT_DIR/dual-write-payload.json"

echo "== Stage 2 tooling: fhir export-ips (synthetic) =="
cargo run -q -p solum-core -- fhir export-ips --out "$SAMPLE_BUNDLE"
echo

echo "== Stage 2 tooling: migrate fhir-import inventory =="
cargo run -q -p solum-core -- migrate fhir-import \
  --bundle "$SAMPLE_BUNDLE" \
  --out "$INVENTORY"
echo "inventory lines: $(wc -l < "$INVENTORY" | tr -d ' ')"
echo

echo "== Stage 2 tooling: migrate dual-write-stub (forced dead-letter) =="
printf '%s\n' '{"resourceType":"Patient","id":"rehearsal-1"}' > "$PAYLOAD"
: > "$DEAD_LETTER"
cargo run -q -p solum-core -- migrate dual-write-stub \
  --payload "$PAYLOAD" \
  --dead-letter "$DEAD_LETTER" \
  --reason "rehearsal_simulated_mirror_fail"
test -s "$DEAD_LETTER"
echo "ok: dead-letter row written (never silent drop)"
echo

echo "== Stages still requiring a live site (not run here) =="
cat <<'EOF'
  [ ] Track B EHRbase up (Solum-Demo: make up-h3 && make smoke-h3)
  [ ] Import inventory rows via sidecar POST /v1/fhir/{type}
  [ ] Prefer: partner UI reads Solum FHIR/AQL for covered domains
  [ ] Cut-over: legal/ops sign-off + CDR backup proven + legacy read-only
EOF
echo

echo "Dry rehearsal tooling steps passed."
echo "Full checklist: docs/MIGRATION-CUTOVER-CHECKLIST.md"
