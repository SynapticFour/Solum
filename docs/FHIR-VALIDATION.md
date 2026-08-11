# FHIR validation — IPS-oriented Patient Summary export

**Date:** 2026-08-11
**Claims allowed:** “Solum’s stage-1 Bundle export passes Solum structural checks; when run through HL7 Validator + `hl7.fhir.uv.ips#2.0.0`, known gaps match documented `ANNAHME`s.”
**Claims forbidden:** Full IPS IG certification, ISiK/TI readiness, clinical correctness.

## Produce the Bundle

```bash
cargo run -q -p solum-example-fhir-ips-export -- \
  examples/fhir-ips-export/out/patient-summary-bundle.json
./scripts/validate-fhir-ips.sh
```

## Structural checks (always run) — **PASS** (2026-08-11)

All Solum-owned checks in `examples/fhir-ips-export/out/structural-check.txt` passed (Bundle document, bdl-9/bdl-10, Composition LOINC `60591-5`, author, Patient / AllergyIntolerance / MedicationStatement / Condition).

## HL7 Validator campaign (2026-08-11)

| Item | Value |
|------|--------|
| JAR | HL7 FHIR Validation tool **6.10.1** (`.cache/validator_cli.jar`, gitignored) |
| Command | `FHIR_VALIDATOR_JAR=.cache/validator_cli.jar ./scripts/validate-fhir-ips.sh` |
| IG | `hl7.fhir.uv.ips#2.0.0` · FHIR R4.0.1 |
| Locale | `de` (Germany) — affects display-name checks |
| Result | **FAILURE**: 7 errors, 5 warnings (script soft-exits 0 unless `SOLUM_FHIR_VALIDATOR_REQUIRE=1`) |

### Errors → `ANNAHME` / follow-up

| Validator finding | Maps to |
|-------------------|---------|
| `fullUrl` must be valid lowercase UUID (`composition-ips`, `patient-…`, …) | Stage-1 uses stable logical ids / non-UUID fullUrls — not claimed UUID URNs |
| Wrong Display Name for LOINC `60591-5` (`Patient summary Document` vs locale `Patient Summary`) | Composition.type display string `ANNAHME`; locale-sensitive |
| AllergyIntolerance rule `ait-1` failed | Minimal allergy row (display-only substance) — incomplete clinical resource |
| (warnings) `dom-6` missing narrative | No `text` narratives emitted — accepted stage-1 omission |

**Honest claim after this campaign:** structural Solum checks PASS; IPS IG validator does **not** pass — gaps are ticketed/known, not hidden.

## Optional re-run

```bash
export FHIR_VALIDATOR_JAR="$PWD/.cache/validator_cli.jar"
# Download once: curl -L -o .cache/validator_cli.jar \
#   https://github.com/hapifhir/org.hl7.fhir.core/releases/latest/download/validator_cli.jar
./scripts/validate-fhir-ips.sh
```

CI / `verify.sh` do **not** require the Java validator.

## Next

German profile landmap: [DE-FHIR-GAP.md](DE-FHIR-GAP.md).
