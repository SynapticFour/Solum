# ADR 0003 — Subject bridge (clinical ↔ genomic)

- **Status:** Accepted (H3.3)
- **Date:** 2026-08-10
- **Product:** Solum (+ Ferrum / BRA consumers)
- **Related:** [ADR 0001](0001-openehr-cdr-and-migration.md), Showcase H2 consent ADR, [MIGRATION-STRANGLER.md](../MIGRATION-STRANGLER.md)

## Context

Sites need a stable link between Solum clinical subjects and Ferrum genomic objects (DRS) without storing BAM/VCF inside Solum. BRA Phenopackets may carry a research identifier that should join the same bridge.

## Decision

Canonical join key: **`solum_subject_id`** (opaque pseudonym string).

| Field | Owner | Notes |
|-------|-------|-------|
| `solum_subject_id` | Solum | Required; used in consent `subject` and CDR subject-link store |
| `ferrum_drs_id` | Ferrum | Optional DRS object id; align with gateway `solum_subject` metadata used for H2.1 teeth |
| `phenopacket_id` | BRA (optional) | Phenopacket resource id when research path is present |
| `ehr_id` | Solum Track B | Optional openEHR EHR id when CDR enabled |

Solum exposes:

- `POST /v1/cdr/subject-link` — upsert (capability `solum:cdr:write`)
- `GET /v1/cdr/subject-link/{solum_subject_id}` — read (`solum:cdr:read`)

Storage: JSONL beside consent store (`--subject-link-store` / default `subject_links.jsonl`).

## Consequences

- Genomic blobs stay in Ferrum; Solum stores only identifiers + audit.
- Ferrum continues to poll consent by `solum_subject` / purpose — operators should use the **same string** as `solum_subject_id`.
- BRA is documentation-linked only for H3 exit; no BRA code change required.

## Non-goals

- Cross-org identity federation / national MPI
- Storing genomic files in Solum
- Automatic Phenopacket ingestion into Ferrum
