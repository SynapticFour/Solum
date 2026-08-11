# Roadmap

Stages are both designed for from the start. **Stage 1 (Track A) and H3 Track B engineering** are implemented; public product language may still say Stage 1 for installable maturity. Remaining gaps are counsel, OPT depth, and optional SaaS preparedness — see Showcase [HORIZON-OPEN-GATES.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/HORIZON-OPEN-GATES.md).

## Stage 1 — Track A (shipped engineering)

Spec-independent controls that already have a clear regulatory floor (e.g. EHDS Annex II security and logging themes):

- Secure processing environment: Crypt4GH field encryption (`solum-crypto`), granular access control, complete audit log (`solum-audit::FileAuditStore` — persistent, hash-chained)
- Consent and access-rights management (`solum-consent` — grant/revoke with purpose binding and full history)
- Compliance evidence **export hooks** (HELIOS-oriented JSON envelope). **Live HELIOS signing is deferred / not productized** — see [helios.md](helios.md)
- Jurisdiction [profile system](profiles.md), `eu-ehds` + `kenya-dpa` (**PROVISIONAL-PRODUCTION-CANDIDATE** — non-counsel Vorprüfung; real counsel still required)
- CustomerHeld CLI / sidecar path; ephemeral keys gated to `dev-local` only
- Optional AWS KMS envelope (`--features aws-kms`); H2.1 Ferrum consent teeth; H2.2 org-IAM

## Track B / H3 — clinical data plane (engineering exit)

Implemented (not a Synaptic Four EHR UI; not MDR clearance):

- EHRbase CDR façade (`/v1/cdr/*`), FHIR subset + AQL proxy, migration inventory + dual-write webhook, subject bridge, partner API docs
- Depth: clinical-modelling honesty, backup runbook, MDCG internal + send pack
- Follow-ons: patient-summary OPT pin; external RA before marketing clinical claims — [H3-CLINICAL-MODELLING.md](H3-CLINICAL-MODELLING.md), counsel package (private)

## H4 / H5 (portfolio)

- **H4 Kenya:** K2 eng done; counsel send + named site still open ([Showcase H4 checklist](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-PILOT-CHECKLIST.md))
- **H5:** optional SaaS-*ready* docs / `SOLUM_TENANT_ID` stamp — not SaaS launch ([H5-KEY-CUSTODY-MANAGED.md](H5-KEY-CUSTODY-MANAGED.md))
- Nigeria / South Africa profiles: DRAFT scaffolds under `config/profiles/planned/` (not auto-loaded); promote after Kenya counsel or commercial reorder
- **Live HELIOS CLI/API signing bridge** — only after HELIOS release + custody story are clear

## Out of scope (default)

Clinical interpretation for diagnosis, therapy, or risk support — see [CONTRIBUTING.md](../CONTRIBUTING.md) and [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §3. Full hospital EHR UI — not planned as Solum.
