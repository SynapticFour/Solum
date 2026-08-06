# ADR 0002 — Track B CDR engine choice (H3 spike)

- **Status:** Accepted
- **Date:** 2026-08-06
- **Product:** Solum
- **Depends on:** [ADR 0001](0001-openehr-cdr-and-migration.md)
- **Spike scope:** Architecture decision only — no production CDR shipped in this change

## Context

ADR 0001 requires an optional openEHR clinical data plane. Three implementation shapes were considered:

1. **Embed** a Rust openEHR RM + persistence inside Solum
2. **Front** a mature open-source CDR (process/sidecar) and keep Solum as compliance + FHIR façade + migration
3. **License** a proprietary CDR (e.g. Better Platform) behind Solum APIs

Constraints: on-prem first, open standards, customer-held keys / Solum audit, MDCG non-device posture, small team, independence from Ferrum.

## Decision

**Default for H3 MVP: Option 2 — Solum fronts EHRbase (Apache 2.0) as the openEHR CDR engine.**

```text
  Partner UI / FHIR clients
           │
           ▼
  ┌────────────────────────────┐
  │ Solum (Track A + façade)   │  consent, Crypt4GH, profiles, audit,
  │  FHIR façade · migration   │  subject bridge, startup residency
  └─────────────┬──────────────┘
                │ openEHR REST (internal)
                ▼
  ┌────────────────────────────┐
  │ EHRbase (+ Postgres)       │  compositions, AQL, RM persistence
  └────────────────────────────┘
```

### Why EHRbase

| Criterion | EHRbase | Embed Rust RM | Better (proprietary) |
|-----------|---------|---------------|----------------------|
| License / sovereignty | Apache 2.0 | Full control, huge build | Proprietary lock-in risk |
| Spec maturity | RM 1.1 + AQL + REST in production use | Years of work | High, but commercial |
| On-prem deploy | JVM + Postgres / Docker | Native binary (attractive) | Vendor stack |
| Team fit | Integration + policy in Rust | Divert Solum from compliance moat | Procurement + SaaS gravity |
| Aligns with “APIs for others to build EHR” | Yes | Yes later | Yes but weaker open story |

### Solum still owns

- Jurisdiction profiles and fail-closed startup
- Consent / purpose binding
- Crypt4GH field policy (and any field-level encrypt before/after CDR write)
- Hash-chained audit of access and migration events
- FHIR façade and strangler adapters ([MIGRATION-STRANGLER.md](../MIGRATION-STRANGLER.md))
- Subject bridge to Ferrum / BRA

### EHRbase owns (as dependency)

- Composition persistence, versioning, AQL execution, openEHR REST

## Consequences

### Positive

- Fastest path to a real openEHR MVP without rewriting the RM in Rust
- Clear partner story: open CDR + Solum compliance layer
- Matches portfolio open-standards doctrine

### Negative / costs

- Second runtime (JVM) beside Solum — ops docs must cover both
- Pin EHRbase version; track their breaking DB migrations
- FHIR bridge: prefer Solum-owned façade first; evaluate EHRbase FHIR bridge as optional later
- Edge/Pi: **EHRbase is hub-class**, not Pi-class — Track B on hub only; Pi stays Track A / Ferrum Edge

### Non-goals for H3 MVP

- Shipping Better Platform as default
- Replacing EHRbase with a from-scratch Rust CDR (revisit only if EHRbase becomes untenable)
- Multi-tenant SaaS CDR (H5)

## Follow-ups (implementation order)

1. **Spike spike:** compose file `Solum-Demo` or Lab Kit profile: Solum sidecar + EHRbase + Postgres (dev-local only)
2. Pin EHRbase Docker tag in Solum `VERSIONS` / docs
3. Minimal write/read composition via Solum façade (one template)
4. Audit event on each façade write
5. Showcase fixture later (Path E+)

## Alternatives rejected (summary)

- **Embed Rust RM now:** heroic; starves Track A hardening and Kenya pack
- **Better as default:** fine as *customer choice* later; not Synaptic Four default (sovereignty + cost)
- **FHIR-only store without openEHR:** rejected in ADR 0001

## Notes

This decision can be revisited after the first EHRbase-backed pilot if ops cost dominates. Revisit trigger: two consecutive pilot failures attributable to EHRbase ops, or license/governance change.
