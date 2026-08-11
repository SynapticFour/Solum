# FHIR IPS export (validator probe input)

**Not a product CLI.** There is no `solum fhir …` command. This example binary (or `solum_fhir::to_fhir_bundle` from a library embed) is the Stage‑1 operator path.

Writes a synthetic IPS-oriented document Bundle via `solum-fhir::to_fhir_bundle`.

```bash
cargo run -q -p solum-example-fhir-ips-export -- examples/fhir-ips-export/out/patient-summary-bundle.json
./scripts/validate-fhir-ips.sh
```

See [docs/FHIR-VALIDATION.md](../../docs/FHIR-VALIDATION.md). Not an ISiK/TI claim.
