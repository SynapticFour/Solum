# ADR 0001 — Optional openEHR clinical data plane + strangler migration

- **Status:** Accepted (architecture) — implementation deferred to H3
- **Date:** 2026-08-06
- **Product:** Solum
- **Related:** [PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md), [MIGRATION-STRANGLER.md](../MIGRATION-STRANGLER.md), portfolio [COORDINATED-PORTFOLIO-ROADMAP](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/COORDINATED-PORTFOLIO-ROADMAP.md)

## Context

Solum Stage 1 is a **compliance sidecar** (consent, Crypt4GH fields, fail-closed residency, hash-chained audit) beside existing EHR/HMIS systems. That path is correct for fast pilots.

Risk: if Solum remains **only** an EHDS/compliance shim, incumbents will absorb “enough” of that surface and the product loses gravity. Customers also increasingly need **clinical + genomic** co-custody under open standards.

Constraint: Synaptic Four will **not** build a full hospital EHR UI. Solum must stay MDCG-safe (no diagnostic/therapeutic interpretation).

`crates/openehr` today is a **stage-2 scaffold** (`STAGE = "2-scaffold"`).

## Decision

Solum adopts an explicit **dual-track** architecture:

| Track | Name | Persistence | Role |
|-------|------|-------------|------|
| **A** | Sidecar / compliance | None (or pointers only) | Wrap legacy; Stage 1 default |
| **B** | Clinical data plane | **openEHR CDR** (compositions) + FHIR façade | Optional standards-native clinical store + APIs for *others* to build EHR UIs |

**Track B is opt-in.** Enabling the CDR does not remove Track A. Sites may run A only, A+B (mirror), or B as system of record for covered domains.

### Storage and APIs (Track B MVP scope)

**In scope for H3 MVP:**

1. Persist openEHR **compositions** for a minimal archetype set (start: patient summary–aligned content; expand by EEHRxF priority categories)
2. **AQL** read API (subset) for compositions
3. **FHIR R4 façade** for a documented resource subset (read + create/update for migration)
4. Same **jurisdiction profiles**, Crypt4GH field policy, consent, and audit as Track A
5. **Subject identity** bridge to Ferrum DRS / BRA Phenopackets (stable pseudonym / local ID — separate short ADR later)

**Out of scope for Solum (permanently unless product strategy changes):**

- Full EHR UI (scheduling, CPOE, billing, nursing charts as a product)
- Diagnostic/risk ML on clinical content
- Owning Ferrum genomic blobs (link only)

### Migration (strangler)

Mandatory product capability — see [MIGRATION-STRANGLER.md](../MIGRATION-STRANGLER.md):

1. **Wrap** — Track A only
2. **Mirror** — dual-write selected resources into CDR
3. **Prefer** — new reads from Solum first
4. **Cut-over** — legacy becomes archive for covered domains

### Independence

- Ferrum remains genomic platform; no Solum tables inside Ferrum.
- HELIOS consumes Solum audit / CDR change evidence; does not embed a CDR.
- Showcase orchestrates demos/verification only.

## Consequences

### Positive

- Durable moat beyond EHDS catch-up
- Clear partner story: “build EHR on Solum APIs”
- Aligns EU/Africa open-standards narrative with Ferrum genomics

### Negative / costs

- CDR ops (backup, schema evolution, archetype governance) become Solum’s problem when Track B is enabled
- openEHR archetype uncertainty remains — MVP must pin a **small, versioned** archetype set and refuse silent drift
- Longer H3 delivery than sidecar-only roadmap

### Follow-ups (implementation, not this ADR)

1. Choose CDR engine approach: embed vs sidecar process vs external openEHR server Solum fronts (decision in H3 spike)
2. Archetype registry pin + CONTRIBUTING rule for additions
3. Subject-ID ADR (Ferrum + Solum + BRA)
4. Showcase Path E+ fixtures for CDR once MVP exists

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Sidecar forever | Moat erosion; no clinical co-custody home |
| Full EHR product | Out of focus; MDR/classification risk; endless UI surface |
| FHIR-only store without openEHR | Weaker long-lived clinical modelling; openEHR already in Stage-2 intent |
| Put clinical data in Ferrum DRS | Wrong regulatory perimeter; couples products |

## Notes

SaaS (portfolio H5) may host Track A and/or B as **managed single-tenant** later; this ADR does not authorize multi-tenant CDR design.
