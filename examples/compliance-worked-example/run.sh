#!/usr/bin/env bash
# Compliance Worked Example — Praxis „Nordlicht“ (EU EHDS profile).
#
# Proves (reproducibly):
#   - CustomerHeld keygen + consent grant → encrypt/decrypt round-trip
#   - Deny A: crypto without --capability fails closed + authorization.denied
#   - Consent revoke updates status
#   - Deny B: decrypt-after-revoke behaviour (documented gap if still allowed)
#   - Hash-chained audit export + verify
#
# Artifacts are kept under artifacts/run-<utc>/ (gitignored).
# No Ferrum, no network, no real PHI.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="$ROOT/config/profiles/eu-ehds.toml"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_DIR="$ROOT/examples/compliance-worked-example/artifacts/run-$STAMP"
mkdir -p "$RUN_DIR"

AUDIT="$RUN_DIR/audit.jsonl"
CONSENT="$RUN_DIR/consent.jsonl"
KEYPAIR="$RUN_DIR/customer.keypair.json"
PLAIN_IN="$RUN_DIR/ehr-patient-summary.txt"
FIELD_OUT="$RUN_DIR/patient_summary.crypt4gh.json"
PLAIN_OUT="$RUN_DIR/ehr-patient-summary.decrypted.txt"
PLAIN_AFTER_REVOKE="$RUN_DIR/ehr-patient-summary.after-revoke.txt"
HELIOS_OUT="$RUN_DIR/helios-export.json"
MANIFEST="$RUN_DIR/MANIFEST.txt"
DENY_A_ERR="$RUN_DIR/deny-a-stderr.txt"
DENY_B_NOTE="$RUN_DIR/deny-b-result.txt"
EVENT_TYPES="$RUN_DIR/event-types.txt"

SUBJECT="patient/WE-001"
ACTOR="practitioner/we-1"
PURPOSE="care_provision"
KEY_REF="customer/nordlicht-we-1"
SOLUM=(cargo run -q -p solum-core --)

echo "== compliance worked example: $RUN_DIR =="

{
  echo "Solum compliance worked example"
  echo "utc: $STAMP"
  echo "commit: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
  echo "profile: $PROFILE"
  if command -v shasum >/dev/null 2>&1; then
    echo "profile_sha256: $(shasum -a 256 "$PROFILE" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    echo "profile_sha256: $(sha256sum "$PROFILE" | awk '{print $1}')"
  else
    echo "profile_sha256: unavailable"
  fi
  echo "subject: $SUBJECT"
  echo "actor: $ACTOR"
  echo "purpose: $PURPOSE"
  echo "key_ref: $KEY_REF"
} >"$MANIFEST"

echo "# Simulated EHR extract for $SUBJECT — not real PHI" >"$PLAIN_IN"
echo "Praxis Nordlicht patient_summary placeholder for $PURPOSE" >>"$PLAIN_IN"

echo "-- 1. check profile =="
"${SOLUM[@]}" check --profile "$PROFILE"

echo "-- 2. crypto keygen (CustomerHeld) =="
"${SOLUM[@]}" crypto keygen --key-ref "$KEY_REF" --out "$KEYPAIR"

echo "-- 3. consent grant =="
"${SOLUM[@]}" consent grant \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject "$SUBJECT" --purpose "$PURPOSE" --actor "$ACTOR" \
  --capability solum:consent:grant \
  --scope patient_summary >/dev/null

STATUS="$("${SOLUM[@]}" consent status \
  --profile "$PROFILE" --consent-store "$CONSENT" \
  --subject "$SUBJECT" --purpose "$PURPOSE")"
test "$STATUS" = "granted" || { echo "FAIL: expected granted, got: $STATUS"; exit 1; }
echo "ok: status=$STATUS"

echo "-- 4/5. encrypt + decrypt round-trip =="
"${SOLUM[@]}" crypto encrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --category patient_summary --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor "$ACTOR" --capability solum:crypto:encrypt \
  --in "$PLAIN_IN" --out "$FIELD_OUT" 2>/dev/null

"${SOLUM[@]}" crypto decrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor "$ACTOR" --capability solum:crypto:decrypt \
  --in "$FIELD_OUT" --out "$PLAIN_OUT" 2>/dev/null
cmp -s "$PLAIN_IN" "$PLAIN_OUT" || { echo "FAIL: decrypt mismatch"; exit 1; }
echo "ok: encrypt/decrypt round-trip"

echo "-- 6. Deny A: encrypt without --capability (fail-closed) =="
set +e
"${SOLUM[@]}" crypto encrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --category patient_summary --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor "$ACTOR" \
  --in "$PLAIN_IN" --out "$RUN_DIR/should-not-exist.crypt4gh.json" \
  >"$RUN_DIR/deny-a-stdout.txt" 2>"$DENY_A_ERR"
DENY_A_RC=$?
set -e
test "$DENY_A_RC" -ne 0 || { echo "FAIL: Deny A should have failed"; exit 1; }
test ! -f "$RUN_DIR/should-not-exist.crypt4gh.json" || {
  echo "FAIL: Deny A wrote ciphertext"
  exit 1
}
grep -E -q 'lacks required capability|solum:crypto:encrypt|authorization' "$DENY_A_ERR" \
  || { echo "FAIL: Deny A stderr missing capability denial"; cat "$DENY_A_ERR"; exit 1; }
if grep -q '"event_type":"authorization.denied"' "$AUDIT" \
  || grep -q '"event_type": "authorization.denied"' "$AUDIT"; then
  echo "ok: Deny A failed closed + authorization.denied in audit"
else
  # JSON may pretty-print; also accept event_type on its own line patterns via python
  python3 - "$AUDIT" <<'PY' || { echo "FAIL: no authorization.denied in audit"; exit 1; }
import json, sys
path = sys.argv[1]
found = False
with open(path) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        ev = json.loads(line)
        # FileAuditStore may nest under "event"
        et = ev.get("event_type") or (ev.get("event") or {}).get("event_type")
        if et == "authorization.denied":
            found = True
            break
sys.exit(0 if found else 1)
PY
  echo "ok: Deny A failed closed + authorization.denied in audit"
fi

echo "-- 7. consent revoke =="
"${SOLUM[@]}" consent revoke \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject "$SUBJECT" --purpose "$PURPOSE" --actor "$ACTOR" \
  --capability solum:consent:revoke >/dev/null

STATUS="$("${SOLUM[@]}" consent status \
  --profile "$PROFILE" --consent-store "$CONSENT" \
  --subject "$SUBJECT" --purpose "$PURPOSE")"
test "$STATUS" = "revoked" || { echo "FAIL: expected revoked, got: $STATUS"; exit 1; }
echo "ok: status=$STATUS"

echo "-- 8. Deny B: decrypt after revoke =="
set +e
"${SOLUM[@]}" crypto decrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --key-ref "$KEY_REF" --keypair "$KEYPAIR" \
  --actor "$ACTOR" --capability solum:crypto:decrypt \
  --in "$FIELD_OUT" --out "$PLAIN_AFTER_REVOKE" 2>"$RUN_DIR/deny-b-stderr.txt"
DENY_B_RC=$?
set -e
if [[ "$DENY_B_RC" -ne 0 ]]; then
  echo "denied" >"$DENY_B_NOTE"
  echo "ok: Deny B — decrypt refused after revoke (enforced)"
else
  {
    echo "gap"
    echo "decrypt_after_revoke=allowed"
    echo "note: encrypt/decrypt check GTM-1 capabilities but do not require active consent.is_granted"
    echo "see docs/WORKED-EXAMPLE.md §Known gaps"
  } >"$DENY_B_NOTE"
  echo "GAP (documented): decrypt after revoke still succeeded — consent not gated on crypto path"
fi

echo "-- 9. audit export + verify =="
"${SOLUM[@]}" audit export --audit "$AUDIT" --out "$HELIOS_OUT"
VERIFY="$("${SOLUM[@]}" audit verify --audit "$AUDIT")"
test "$VERIFY" = "ok" || { echo "FAIL: audit verify: $VERIFY"; exit 1; }
echo "ok: audit chain verified"

python3 - "$AUDIT" "$EVENT_TYPES" <<'PY'
import json, sys
from collections import Counter
path, out = sys.argv[1], sys.argv[2]
counts = Counter()
with open(path) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        ev = json.loads(line)
        et = ev.get("event_type") or (ev.get("event") or {}).get("event_type")
        if et:
            counts[et] += 1
with open(out, "w") as o:
    for k in sorted(counts):
        o.write(f"{k}\t{counts[k]}\n")
print("event types:")
for k in sorted(counts):
    print(f"  {k}: {counts[k]}")
PY

# Convenience pointer for docs / latest run
ln -sfn "run-$STAMP" "$ROOT/examples/compliance-worked-example/artifacts/latest"

echo "ok: compliance worked example passed"
echo "artifacts: $RUN_DIR"
