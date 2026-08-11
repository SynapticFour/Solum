# FHIR IPS export (validator probe input)

Writes a synthetic IPS-oriented document Bundle via `solum-fhir::to_fhir_bundle`.

```bash
cargo run -q -p solum-example-fhir-ips-export -- examples/fhir-ips-export/out/patient-summary-bundle.json
./scripts/validate-fhir-ips.sh
```

See [docs/FHIR-VALIDATION.md](../../docs/FHIR-VALIDATION.md). Not an ISiK/TI claim.
