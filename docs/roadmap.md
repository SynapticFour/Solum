# Roadmap

Stages are both designed for from the start; **only stage 1 is under active implementation**.

## Stage 1 — build now

Spec-independent controls that already have a clear regulatory floor (e.g. EHDS Annex II security and logging themes):

- Secure processing environment: encryption at rest / in transit, granular access control, complete audit log
- Consent and access-rights management (access, who accessed, onward sharing, rectification / completion — anticipating EEHRxF individual rights where implementable without waiting for every implementing act)
- Signed, reproducible compliance evidence hooks (HELIOS-oriented; see [helios.md](helios.md))
- Jurisdiction [profile system](profiles.md), initially `eu-ehds`, schema ready for further countries

## Stage 2 — planned

Communicated as evolving with specifications and demand — not implemented as stage-1 scope:

- FHIR / IHE interoperability depth for EEHRxF priority categories (e.g. patient summary, laboratory results, discharge reports, imaging reports / manifests, prescriptions)
- openEHR layer for deeper clinical semantics and long-lived modelling
- SaaS operating model built on stage-1 key-custody and audit foundations
- Additional jurisdiction profiles (e.g. Nigeria, South Africa, others) as data files when needed

## Out of scope for both stages (default)

Clinical interpretation for diagnosis, therapy, or risk support — see [CONTRIBUTING.md](../CONTRIBUTING.md) and [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §3.
