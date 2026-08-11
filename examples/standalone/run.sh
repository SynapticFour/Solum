#!/usr/bin/env bash
# Mode A — Standalone reference deployment (no Ferrum).
#
# Frame: a hospital / EHR operator already stores clinical records in their
# own database. Solum sits beside that system as a compliance layer:
#   - grant/check purpose-bound consent
#   - Crypt4GH-encrypt a clinical field category before it leaves the trust
#     boundary (here: a plain file standing in for a DB blob)
#   - retain a hash-chained audit trail and verify it
#
# No Ferrum crates, APIs, or network calls are used. Keys use the CLI
# CustomerHeld --keypair path (evaluation posture; not an HSM).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="$ROOT/config/profiles/eu-ehds.toml"
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/solum-standalone.XXXXXX")"
AUDIT="$WORKDIR/audit.jsonl"
CONSENT="$WORKDIR/consent.jsonl"
KEYPAIR="$WORKDIR/customer.keypair.json"
PLAIN_IN="$WORKDIR/ehr-patient-summary.txt"
FIELD_OUT="$WORKDIR/patient_summary.crypt4gh.json"
PLAIN_OUT="$WORKDIR/ehr-patient-summary.decrypted.txt"
HELIOS_OUT="$WORKDIR/helios-export.json"
KEY_REF="customer/standalone-1"

cleanup() {
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

echo "== standalone (Mode A): workdir $WORKDIR =="
echo "# Simulated EHR extract for patient/42 — not real PHI" > "$PLAIN_IN"
echo "patient_summary placeholder for care_provision" >> "$PLAIN_IN"

echo "-- check profile against runtime =="
cargo run -q -p solum-core -- check --profile "$PROFILE"

echo "-- consent grant (as if EHR called Solum after clinician request) =="
cargo run -q -p solum-core -- consent grant \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor practitioner/7 \
  --capability solum:consent:grant \
  --scope patient_summary >/dev/null

echo "-- consent status =="
STATUS="$(cargo run -q -p solum-core -- consent status \
  --profile "$PROFILE" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision)"
test "$STATUS" = "granted" || { echo "FAIL: expected granted, got: $STATUS"; exit 1; }
echo "ok: status=$STATUS"

echo "-- crypto keygen (CustomerHeld operator file) =="
cargo run -q -p solum-core -- crypto keygen \
  --key-ref "$KEY_REF" --out "$KEYPAIR"

echo "-- crypto encrypt (DB blob → Crypt4GH EncryptedField JSON) =="
cargo run -q -p solum-core -- crypto encrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --category patient_summary --subject patient/42 --purpose care_provision \
  --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor practitioner/7 --capability solum:crypto:encrypt \
  --in "$PLAIN_IN" --out "$FIELD_OUT" 2>/dev/null

echo "-- crypto decrypt (round-trip back into EHR staging file) =="
cargo run -q -p solum-core -- crypto decrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision \
  --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor practitioner/7 --capability solum:crypto:decrypt \
  --in "$FIELD_OUT" --out "$PLAIN_OUT" 2>/dev/null
cmp -s "$PLAIN_IN" "$PLAIN_OUT" || { echo "FAIL: decrypt mismatch"; exit 1; }
echo "ok: encrypt/decrypt round-trip"

echo "-- audit export + verify =="
cargo run -q -p solum-core -- audit export --audit "$AUDIT" --out "$HELIOS_OUT"
VERIFY="$(cargo run -q -p solum-core -- audit verify --audit "$AUDIT")"
test "$VERIFY" = "ok" || { echo "FAIL: audit verify: $VERIFY"; exit 1; }
echo "ok: audit chain verified"

echo "ok: standalone reference deployment (Mode A) passed"
