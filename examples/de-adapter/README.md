# KIS Patient adapter (not a hospital EHR)

`solum_fhir::to_kis_patient_adapter` maps Solum [`PatientInfo`](../../crates/fhir/src/patient_summary.rs) to a FHIR R4 Patient JSON slice a German KIS typically accepts for identifier / name / birthDate.

It is an **adapter**, not a HIS, not ISiK-validated, and not a Medizinprodukt.

## What it does

- Copies `id`, `identifier`, `name`, `birthDate`.
- Uses `http://fhir.de/sid/gkv/kvid-10` when the operator already stamped that system.
- Falls back to `https://synapticfour.com/fhir/sid/local-patient` when no identifier is present.
- Tags `meta.tag` with `kis-patient-v0`. Does **not** set `meta.profile` (no IG instance claim).

## What it does not

- SMC-B / TI / Fachdienst authentication
- ISiK Basis Patient required elements (gender, German extensions, full identifier cardinality)
- epa write, full ISiK module set, MII Kerndatensatz as an EHR

Full ISiK IG mapping stays [pilot-gated](../../docs/DE-ADAPTER-SPIKE.md). Gap table: [DE-FHIR-GAP.md](../../docs/DE-FHIR-GAP.md).

## Prove

```bash
cargo test -p solum-fhir --lib kis_patient_adapter
```

Library use (from another crate that depends on `solum-fhir`):

```rust
use solum_fhir::{to_kis_patient_adapter, PatientInfo, DE_KVID10_SYSTEM};

let json = to_kis_patient_adapter(&patient);
assert!(json.get("meta").unwrap().get("profile").is_none());
```

Hospital identity (Keycloak / Entra / SMART Backend Services, not App Launch): [AUTH-HOSPITAL.md](../../docs/AUTH-HOSPITAL.md).
