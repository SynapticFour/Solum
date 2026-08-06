# Roadmap

Stages are both designed for from the start; **only stage 1 is under active implementation**.

## Stage 1 — build now

Spec-independent controls that already have a clear regulatory floor (e.g. EHDS Annex II security and logging themes):

- Secure processing environment: encryption at rest / in transit (`solum-crypto` policy layer implemented; actual field-level encryption still open — see below), granular access control, complete audit log (`solum-audit::FileAuditStore` — persistent, hash-chained, implemented)
- Consent and access-rights management (`solum-consent` — grant/revoke with purpose binding and full history, implemented; anticipating EEHRxF individual rights — access, who accessed, onward sharing, rectification / completion — where implementable without waiting for every implementing act)
- Compliance evidence **export hooks** (HELIOS-oriented JSON envelope from `solum-audit` — hash-chained, operator-verifiable). **Live HELIOS signing is deferred / not productized** — see [helios.md](helios.md)
- Jurisdiction [profile system](profiles.md), initially `eu-ehds`, schema ready for further countries; Kenya is **PROVISIONAL-PRODUCTION-CANDIDATE** (non-counsel Vorprüfung; real counsel still required)
- CustomerHeld CLI operator path (`crypto keygen` + `--keypair`) for Stage‑1 evaluations; ephemeral keys gated to `dev-local` only

## Stage 2 — planned

Communicated as evolving with specifications and demand — not implemented as stage-1 scope:

- FHIR / IHE interoperability depth for EEHRxF priority categories (e.g. patient summary, laboratory results, discharge reports, imaging reports / manifests, prescriptions)
- **openEHR clinical data plane (Track B)** — see [ADR 0001](adr/0001-openehr-cdr-and-migration.md), [ADR 0002 EHRbase](adr/0002-cdr-engine-ehrbase.md), and [MIGRATION-STRANGLER.md](MIGRATION-STRANGLER.md) (architecture + engine choice accepted; CDR MVP not built yet)
- SaaS *preparedness* (tenancy / key boundaries) on stage-1 foundations — not SaaS as default delivery
- Additional jurisdiction profiles; **Kenya pack hardening** first among African profiles (see Showcase [H4 geography decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md) and [counsel/KENYA-K1-BRIEF.md](counsel/KENYA-K1-BRIEF.md))
- **Live HELIOS CLI/API signing bridge** (only after HELIOS release + custody story are clear — not claimed in Stage 1)

## Out of scope for both stages (default)

Clinical interpretation for diagnosis, therapy, or risk support — see [CONTRIBUTING.md](../CONTRIBUTING.md) and [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §3.
