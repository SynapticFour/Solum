# FHIR validation — IPS-oriented Patient Summary export

**Date:** 2026-08-11
**Claims allowed:** “Solum’s stage-1 Bundle export passes Solum structural checks **and** HL7 Validator + `hl7.fhir.uv.ips#2.0.0` with 0 errors / 0 warnings (campaign below).”
**Claims forbidden:** Full IPS IG certification beyond this package pin, ISiK/TI readiness, clinical correctness.

## Produce the Bundle

```bash
cargo run -q -p solum-example-fhir-ips-export -- \
  examples/fhir-ips-export/out/patient-summary-bundle.json
./scripts/validate-fhir-ips.sh
```

**Operator path:** there is **no** `solum fhir …` product CLI yet. Use the example binary above (or call `solum_fhir::to_fhir_bundle` from a library embed). See [examples/fhir-ips-export/README.md](../examples/fhir-ips-export/README.md).

## Structural checks — **PASS**

All Solum-owned checks in `examples/fhir-ips-export/out/structural-check.txt` pass.

## HL7 Validator campaign

| Item | 2026-08-11 (before Bundle harden) | 2026-08-11 (after UUID / LOINC / ait-1 / narrative) |
|------|-----------------------------------|------------------------------------------------------|
| JAR | 6.10.1 | 6.10.1 |
| IG | `hl7.fhir.uv.ips#2.0.0` | same |
| Result | **FAIL** — 7 errors, 5 warnings | **Success** — 0 errors, 0 warnings, 1 note (`Alles OK`) |

### What we fixed

| Prior error | Fix |
|-------------|-----|
| Invalid `urn:uuid:…` fullUrls | Deterministic UUID v5 (`urn:uuid:<uuid>`) for all entry fullUrls + Bundle.identifier |
| LOINC `60591-5` wrong display | Official display **Patient Summary** |
| AllergyIntolerance `ait-1` | Emit `clinicalStatus=active` |
| `dom-6` narrative warnings | Generated `text.div` on Composition / Patient / clinical resources |

Remaining `ANNAHME`s (terminology binding, MedicationRequest path, display-only author Reference, provisional MII URL) are **not** claimed resolved — they simply did not fail this IPS package run.

## Re-run

```bash
export FHIR_VALIDATOR_JAR="$PWD/.cache/validator_cli.jar"
./scripts/validate-fhir-ips.sh
```

CI / `verify.sh` do not require the Java validator.

## Next

German profile landmap: [DE-FHIR-GAP.md](DE-FHIR-GAP.md).
