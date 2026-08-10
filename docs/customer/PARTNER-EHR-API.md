# Partner API — build an EHR UI on Solum (H3.4)

Solum provides **APIs**, not a hospital EHR UI. Partners (or site IT) build clinical UX against these surfaces while Solum owns compliance, audit, and optional openEHR persistence.

**Auth:** every route requires `X-Solum-Sidecar-Token`. Mutating routes need GTM-1 capabilities (or H2.2 org-IAM Bearer JWT → CAP_*).

## Capability strings

| Capability | Use |
|------------|-----|
| `solum:consent:*` / `solum:crypto:*` | Track A Stage-1 |
| `solum:cdr:write` | CDR, FHIR create, subject-link upsert, template upload |
| `solum:cdr:read` | Composition/FHIR/AQL/subject-link read |

## Track B — openEHR CDR

| Method | Path |
|--------|------|
| POST | `/v1/cdr/template` |
| POST | `/v1/cdr/ehr` |
| POST | `/v1/cdr/ehr/{ehr_id}/composition` |
| GET | `/v1/cdr/ehr/{ehr_id}/composition/{uid}` |
| POST | `/v1/cdr/aql` body `{ "q": "SELECT … COMPOSITION …", "actor", "capability" }` |

Requires `--ehrbase-url`. See [H3-EHRBASE-SPIKE.md](../H3-EHRBASE-SPIKE.md).

## FHIR façade (H3.1 subset)

Allowlisted: `Bundle`, `Composition`, `Patient`, `AllergyIntolerance`, `MedicationStatement`, `Condition`.

| Method | Path |
|--------|------|
| POST | `/v1/fhir/{resourceType}` body `{ actor, capability, resource, link_cdr? }` |
| GET | `/v1/fhir/{resourceType}/{id}?actor=&capability=` |

`POST /v1/fhir/Patient` also upserts subject-link (`solum_subject_id = Patient.id`). FHIR→CDR is **co-create** with the pinned OPT — see [H3-CLINICAL-MODELLING.md](../H3-CLINICAL-MODELLING.md).

IPS document shape can be produced with the `solum-fhir` library (`to_fhir_bundle`) and POSTed as `Bundle`.

## Subject bridge (H3.3)

| Method | Path |
|--------|------|
| POST | `/v1/cdr/subject-link` |
| GET | `/v1/cdr/subject-link/{solum_subject_id}` |

Contract: [ADR 0003](../adr/0003-subject-bridge.md). Align Ferrum DRS metadata `solum_subject` with `solum_subject_id`.

## Migration

Batch inventory: `solum migrate fhir-import`. Live dual-write webhook: `POST /v1/migrate/dual-write` (`201` mirrored / `202` dead-lettered). Offline sim: `solum migrate dual-write-stub`. Operator cut-over: [MIGRATION-CUTOVER-CHECKLIST.md](../MIGRATION-CUTOVER-CHECKLIST.md). Ops: [H3-EHRBASE-BACKUP.md](../H3-EHRBASE-BACKUP.md).

## Honesty / MDCG

- Solum does **not** interpret clinical data for diagnosis, therapy, or risk (see CONTRIBUTING / PRODUCT-DEFINITION).
- Evidence Packs and audit exports are **not** MDR/EHDS certification.
- External RA send pack: [H3-MDCG-SEND-CHECKLIST.md](../counsel/H3-MDCG-SEND-CHECKLIST.md).
- Edge/Pi: EHRbase is hub-class only.
