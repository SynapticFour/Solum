#!/usr/bin/env bash
# Runnable Stage-1 claims proof trail (Track A + structural FHIR + Kenya).
# Optional: HL7 Validator JAR, Track B H3 (prints pointer only).
#
# Usage: ./scripts/demo-claims-proof.sh
# Env:
#   FHIR_VALIDATOR_JAR — if set and present, run Java IPS validator
#   SOLUM_SKIP_KENYA_CLI=1 — skip kenya check demos
#   SOLUM_SKIP_FHIR=1 — skip FHIR export/validate
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "======== Solum claims proof trail ========"
echo "repo: $ROOT"
echo "HEAD: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "doc:  docs/CLAIMS-PROOF-TRAIL.md"
echo

echo "== A1–A3 / A6: compliance worked example (Deny A/B + audit export) =="
./examples/compliance-worked-example/run.sh
echo

if [ "${SOLUM_SKIP_FHIR:-0}" != "1" ]; then
  echo "== A9 (+ optional A10): FHIR IPS export / validate =="
  ./scripts/validate-fhir-ips.sh
  echo
else
  echo "== A9/A10: skipped (SOLUM_SKIP_FHIR=1) =="
fi

if [ "${SOLUM_SKIP_KENYA_CLI:-0}" != "1" ]; then
  echo "== A8: Kenya check — KE + CustomerHeld should pass =="
  SOLUM_STORAGE_REGION=KE cargo run -q -p solum-core -- \
    check --profile config/profiles/kenya-dpa.toml
  echo

  echo "== A8: Kenya check — EU region must refuse =="
  set +e
  SOLUM_STORAGE_REGION=EU cargo run -q -p solum-core -- \
    check --profile config/profiles/kenya-dpa.toml
  ke_eu=$?
  set -e
  if [ "$ke_eu" -eq 0 ]; then
    echo "FAIL: kenya-dpa accepted EU storage_region"
    exit 1
  fi
  echo "ok: kenya-dpa refused EU storage_region (exit $ke_eu)"
  echo

  echo "== A8: Kenya transfer destinations fail-closed (unit) =="
  cargo test -q -p solum-profiles kenya_validate_transfer_fail_closed_empty_destinations
  echo "ok: kenya_validate_transfer_fail_closed_empty_destinations"
  echo
else
  echo "== A8: skipped (SOLUM_SKIP_KENYA_CLI=1) =="
fi

echo "== A12: Track B H3 (optional Docker — not run here) =="
echo "  cd ../Solum-Demo && make up-h3 && make smoke-h3"
echo "  see docs/H3-WORKED-EVIDENCE.md"
echo

echo "== A11: DE gap dossier (document-only) =="
echo "  docs/DE-FHIR-GAP.md"
echo

echo "== A13: planned Nigeria/SA scaffolds (not auto-loaded) =="
ls config/profiles/planned/*.toml
echo

echo "All local claims-proof steps passed."
echo "Full baseline: ./scripts/verify.sh"
echo "Claims map:    docs/CLAIMS-PROOF-TRAIL.md"
