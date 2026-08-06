# Solum — strangler migration (legacy → Track B)

**Status:** Design (H3) — implements the path in [ADR 0001](adr/0001-openehr-cdr-and-migration.md)
**Audience:** integrators, pilot operators, Synaptic Four engineering

This is **not** a claim that migration is automated today. Stage 1 ships Track A (sidecar). Track B CDR is planned.

---

## Goal

Let sites **start with Solum beside** an existing EHR/HMIS, then **move clinical system-of-record for covered domains into Solum’s openEHR CDR**, until the legacy system is archive/optional for those domains — without Synaptic Four shipping a full EHR UI.

Genomic data stays in **Ferrum**; subjects are linked, not duplicated as BAM/VCF inside Solum.

---

## Stages

```text
  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
  │  1 Wrap  │ ──► │ 2 Mirror │ ──► │ 3 Prefer │ ──► │ 4 Cut-over│
  └──────────┘     └──────────┘     └──────────┘     └──────────┘
   Track A only     A + dual-write   read Solum 1st    SoR = Solum
```

### Stage 1 — Wrap (today / H1)

| Item | Detail |
|------|--------|
| Solum role | Sidecar: consent, Crypt4GH fields, audit, residency |
| Data home | Legacy EHR remains system of record |
| Success | Fail-closed authz; purpose binding; Evidence Pack possible |
| Exit criteria | H1 pilot checklist signed for sidecar |

### Stage 2 — Mirror (H3)

| Item | Detail |
|------|--------|
| Solum role | Track A + Track B CDR enabled |
| Data flow | Dual-write: selected FHIR resources / compositions into CDR on create/update from adapter |
| Reads | Still primarily legacy; Solum used for compliance + optional second read |
| Tooling | Import job (batch FHIR → CDR); webhook/adapter for incremental dual-write |
| Exit criteria | Round-trip verify: N patients mirrored; audit events for each write; rollback = disable dual-write |

### Stage 3 — Prefer (H3+)

| Item | Detail |
|------|--------|
| Reads | New apps / partner UI / reports **prefer Solum** FHIR/AQL APIs |
| Writes | New workflows write Solum first; legacy updated via reverse sync **or** accepted lag |
| Success | Clinical users can complete agreed workflows without opening legacy for those domains |
| Exit criteria | Documented domain list “Solum-preferred”; latency/error SLOs agreed |

### Stage 4 — Cut-over (H3+/H4 site-dependent)

| Item | Detail |
|------|--------|
| System of record | Solum CDR for covered domains |
| Legacy | Read-only archive or decommissioned for those domains |
| Genomics | Ferrum links remain; consent revoke still blocks purpose-bound access |
| Exit criteria | Legal/ops sign-off; backup/restore of CDR proven; reverse sync frozen |

---

## Domain coverage (suggested order)

Align with EEHRxF-oriented priorities; expand only with pinned archetypes:

1. Patient identity / summary (link to Ferrum subject)
2. Consent / legal basis artefacts (already Track A)
3. Laboratory results
4. Discharge / imaging reports (later)
5. Prescriptions / dispensation (later)

Do **not** expand domains without: archetype pin, FHIR mapping table, migration test fixtures, counsel note if jurisdiction-sensitive.

---

## Adapter shapes (contracts)

| Adapter | Direction | Notes |
|---------|-----------|-------|
| `fhir-import` | Legacy → Solum | Batch; idempotent by resource id |
| `fhir-dual-write` | Legacy write path → Solum | Best-effort + dead-letter queue |
| `fhir-façade` | Clients → Solum | Public partner API |
| `aql-read` | Clients → Solum | openEHR query subset |
| `subject-bridge` | Solum ↔ Ferrum/BRA | Pseudonym / Phenopacket id |

All adapters emit Solum audit events. Failures are visible; silent drop is forbidden.

---

## What we will not automate away

- Institutional change management and training
- Full historical chart conversion in one shot
- Replacing billing/scheduling modules inside Solum
- Claiming legal “EHR certification” from migration alone

---

## Relation to Kenya / other jurisdictions (H4)

Migration stages are **jurisdiction-agnostic**. Profile TOML still gates residency, retention, and consent purposes. A Kenya (or other) pack must be **production-ready** before Stage 4 cut-over in that country — Stage 1–2 wrap/mirror may use draft profiles only in evaluation sandboxes, never as production SoR.
