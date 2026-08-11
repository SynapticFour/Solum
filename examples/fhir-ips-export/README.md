# FHIR IPS export (validator probe input)

Writes a synthetic IPS-oriented document Bundle via `solum-fhir::to_fhir_bundle`.

**Preferred CLI:** `solum fhir export-ips --out <path>` (same Bundle shape). This example binary remains for library-shaped demos.

```bash
cargo run -q -p solum-core -- fhir export-ips --out examples/fhir-ips-export/out/patient-summary-bundle.json
# or:
cargo run -q -p solum-example-fhir-ips-export -- examples/fhir-ips-export/out/patient-summary-bundle.json
./scripts/validate-fhir-ips.sh
```

See [docs/FHIR-VALIDATION.md](../../docs/FHIR-VALIDATION.md). Not an ISiK/TI claim.
