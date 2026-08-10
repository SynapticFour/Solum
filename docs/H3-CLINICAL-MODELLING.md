# H3 clinical modelling honesty

**Status:** Engineering honesty note (2026-08-10)
**Pins:** [`VERSIONS`](../VERSIONS) · fixture OPT [`crates/openehr/fixtures/minimal_observation.opt`](../crates/openehr/fixtures/minimal_observation.opt)

## What is pinned today

| Item | Value |
|------|--------|
| OPT / template id | `minimal_observation.en.v1` |
| FHIR façade allowlist | Patient, Composition, AllergyIntolerance, MedicationStatement, Condition (+ Bundle import) |
| FHIR → openEHR | **Co-create**, not semantic map: façade stores FHIR JSONL; optional CDR commit uses the pinned example composition for presence smoke |
| Patient → subject bridge | `POST /v1/fhir/Patient` auto-upserts `/v1/cdr/subject-link` with `solum_subject_id = Patient.id` |

Correlation for partners: audit events (`cdr.fhir.created`, `cdr.subject_link.upserted`) and subject-link `ehr_id` when Track B is on — **not** a full openEHR archetype projection of FHIR fields.

## What is deliberately not claimed

- No International Patient Summary (IPS) OPT pin yet
- No lossless FHIR R4 ↔ openEHR RM bidirectional transform
- No clinical decision / scoring / enrichment on mapped fields (MDCG posture)

## Next pin (post-H3 engineering)

1. Choose and vendor an openEHR **patient-summary** (or site-agreed) OPT under Apache-2.0 / CKM licence terms.
2. Replace `SOLUM_H3_TEMPLATE_ID` + fixture; keep `minimal_observation` as regression fixture only.
3. Add field-level mapping table (FHIR element → OPT path) reviewed under [H3-MDCG-INTERNAL-REVIEW.md](counsel/H3-MDCG-INTERNAL-REVIEW.md) — still no inference.

Until then, greenfield pilots should treat Solum as **custody + evidence + join keys**, and render clinical UI from FHIR façade JSON (or partner EHR) rather than from openEHR AQL alone.
