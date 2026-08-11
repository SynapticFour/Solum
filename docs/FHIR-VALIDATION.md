# FHIR validation — IPS-oriented Patient Summary export

**Date:** 2026-08-11
**Claims allowed:** “Solum’s stage-1 Bundle export passes Solum structural checks (bdl-9/bdl-10, Composition/Patient/sections) and can be fed to the HL7 Validator with the IPS package.”
**Claims forbidden:** Full IPS IG certification, ISiK/TI readiness, clinical correctness.

## Produce the Bundle

```bash
cargo run -q -p solum-example-fhir-ips-export -- \
  examples/fhir-ips-export/out/patient-summary-bundle.json
```

Or:

```bash
./scripts/validate-fhir-ips.sh
```

## Structural checks (always run)

`scripts/validate-fhir-ips.sh` writes `examples/fhir-ips-export/out/structural-check.txt`.

Expected **PASS** rows:

| Check | Maps to |
|-------|---------|
| `resourceType=Bundle`, `type=document` | Document Bundle |
| bdl-9 identifier system/value | FHIR R4 document Bundle invariants |
| bdl-10 timestamp | FHIR R4 document Bundle invariants |
| Composition first + LOINC `60591-5` | IPS document type (`ANNAHME` in `patient_summary.rs`) |
| Composition.author present | R4 1..\* |
| Patient / AllergyIntolerance / MedicationStatement / Condition | Stage-1 section entries |

## Optional HL7 Validator (IPS package)

1. Download [HL7 FHIR Validator](https://github.com/hapifhir/org.hl7.fhir.core/releases) `validator_cli.jar`.
2. Run:

```bash
export FHIR_VALIDATOR_JAR=/path/to/validator_cli.jar
# optional pin; default hl7.fhir.uv.ips#2.0.0
export FHIR_IPS_IG=hl7.fhir.uv.ips#2.0.0
./scripts/validate-fhir-ips.sh
```

Log: `examples/fhir-ips-export/out/validator-log.txt`.
Set `SOLUM_FHIR_VALIDATOR_REQUIRE=1` to fail when the JAR is missing or the validator exits non-zero.

CI / `verify.sh` do **not** require the Java validator (network + JDK). Structural export is available anytime via the script.

## Known `ANNAHME` → likely validator friction

From [`crates/fhir/src/patient_summary.rs`](../crates/fhir/src/patient_summary.rs):

| Assumption | Likely IPS IG impact |
|------------|----------------------|
| Composition.type LOINC `60591-5` | Confirm against targeted IPS STU |
| Section emptyReason `nilknown` | IPS prefers “known absent” clinical resources |
| MedicationStatement only | No MedicationRequest path |
| Display-only clinical codes | No SNOMED/IPS value-set binding |
| Author as display-only Reference | Missing Organization entry / `reference` URL |
| Provisional MII extension URL | Not a jointly agreed StructureDefinition |

Map concrete validator errors into this table when you run with `FHIR_VALIDATOR_JAR`.

## Next

German profile landmap and reference probes: [DE-FHIR-GAP.md](DE-FHIR-GAP.md).
