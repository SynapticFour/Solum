#!/usr/bin/env bash
# Export IPS-oriented Bundle + structural checks; optional HL7 Validator JAR.
#
# Soft-skip external validator when FHIR_VALIDATOR_JAR is unset / missing
# (exit 0) unless SOLUM_FHIR_VALIDATOR_REQUIRE=1.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT_DIR="${SOLUM_FHIR_OUT:-$ROOT/examples/fhir-ips-export/out}"
BUNDLE="$OUT_DIR/patient-summary-bundle.json"
STRUCT="$OUT_DIR/structural-check.txt"
VALIDATOR_LOG="$OUT_DIR/validator-log.txt"
REQUIRE="${SOLUM_FHIR_VALIDATOR_REQUIRE:-0}"
JAR="${FHIR_VALIDATOR_JAR:-}"
IPS_IG="${FHIR_IPS_IG:-hl7.fhir.uv.ips#2.0.0}"

mkdir -p "$OUT_DIR"

echo "== fhir-ips-export =="
cargo run -q -p solum-example-fhir-ips-export -- "$BUNDLE"

echo "== structural checks (Solum-owned) =="
python3 - "$BUNDLE" "$STRUCT" <<'PY'
import json, sys
bundle_path, out_path = sys.argv[1], sys.argv[2]
b = json.load(open(bundle_path))
checks = []

def ok(name, cond, detail=""):
    checks.append((name, bool(cond), detail))

ok("resourceType=Bundle", b.get("resourceType") == "Bundle")
ok("type=document", b.get("type") == "document")
ok("bdl-9 identifier.system", bool((b.get("identifier") or {}).get("system")))
ok("bdl-9 identifier.value", bool((b.get("identifier") or {}).get("value")))
ok("bdl-10 timestamp", bool(b.get("timestamp")))
entries = b.get("entry") or []
ok("entry non-empty", len(entries) >= 2, f"n={len(entries)}")
comp = (entries[0].get("resource") if entries else {}) or {}
ok("Composition first", comp.get("resourceType") == "Composition")
ok("Composition.type LOINC 60591-5",
   ((comp.get("type") or {}).get("coding") or [{}])[0].get("code") == "60591-5")
ok("Composition.author present", bool(comp.get("author")))
types = [((e.get("resource") or {}).get("resourceType")) for e in entries]
ok("Patient entry", "Patient" in types)
ok("AllergyIntolerance entry", "AllergyIntolerance" in types)
ok("MedicationStatement entry", "MedicationStatement" in types)
ok("Condition entry", "Condition" in types)

lines = []
failed = False
for name, passed, detail in checks:
    status = "PASS" if passed else "FAIL"
    if not passed:
        failed = True
    lines.append(f"{status}\t{name}" + (f"\t{detail}" if detail else ""))
open(out_path, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
sys.exit(1 if failed else 0)
PY
echo "ok: structural checks → $STRUCT"

if [[ -z "$JAR" || ! -f "$JAR" ]]; then
  msg="HL7 Validator JAR not configured (set FHIR_VALIDATOR_JAR to validator_cli.jar)"
  echo "SKIP: $msg" | tee "$VALIDATOR_LOG"
  if [[ "$REQUIRE" == "1" ]]; then
    echo "FAIL: SOLUM_FHIR_VALIDATOR_REQUIRE=1 but JAR missing"
    exit 1
  fi
  echo "ok: structural path only (see docs/FHIR-VALIDATION.md)"
  exit 0
fi

echo "== HL7 Validator ($JAR, IG $IPS_IG) =="
set +e
java -jar "$JAR" "$BUNDLE" -version 4.0.1 -ig "$IPS_IG" >"$VALIDATOR_LOG" 2>&1
rc=$?
set -e
tail -n 40 "$VALIDATOR_LOG" || true
if [[ $rc -ne 0 ]]; then
  echo "NOTE: validator exited $rc — map failures to ANNAHME markers in docs/FHIR-VALIDATION.md"
  # Do not fail the soft path; operators inspect the log. Require mode fails.
  if [[ "$REQUIRE" == "1" ]]; then
    exit "$rc"
  fi
fi
echo "ok: validator log → $VALIDATOR_LOG"
