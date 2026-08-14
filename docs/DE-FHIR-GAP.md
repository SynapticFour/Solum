# DE FHIR / TI reference — gap dossier

**Date:** 2026-08-11
**Purpose:** Competence signal + honest roadmap — Solum’s FHIR surface mapped against **public** German reference expectations.
**Claims allowed:** “We compared Solum’s IPS-oriented export to ISiK / gematik-facing FHIR expectations and documented gaps.”
**Claims forbidden:** “TI-konform”, “gematik-zertifiziert”, “ISiK-ready”, epa connectivity, SMC-B / Fachdienst auth.

## Landmap — what Solum exports vs DE references

| Layer | Solum today | Typical DE reference | Status |
|-------|-------------|----------------------|--------|
| Document shape | IPS-oriented FHIR R4 **document Bundle** (`solum-fhir`) | Often ISiK / KBV / epa **profiled** resources; not raw UV IPS | `fail` / mismatch by design |
| Patient | Minimal Patient in Bundle (id, name, birthDate, identifier) | **ISiK Basis Patient** (identifier systems, gender, required elements, German extensions) | `fail` |
| Composition | IPS LOINC `60591-5` + three sections | DE document types / ISiK document profiles differ | `fail` |
| Allergies / Meds / Problems | Display-text entries | Bound value sets / ISiK observation & medication profiles | `fail` |
| Consent / crypto / audit | Solum CLI + sidecar (Track A) | Orthogonal to FHIR IG; DE sites still need policy/evidence | `pass` (Solum moat) |
| Exchange auth | Sidecar shared secret + **org-IAM required** on pilot profiles | TI identity (SMC-B, OIDC Fachdienst, …) | `n/a` (out of Stage-1 scope) |
| openEHR Track B | EHRbase façade | Separate from TI FHIR; useful for CDR pilots | `n/a` to TI claim |

Public starting points (operator research; URLs evolve):

- ISiK Implementation Guides (HL7 DE / gematik ecosystem) — Basis / Patient / Dokumentenaustausch
- gematik FHIR / Referenzumgebung documentation for the target Fachanwendung
- HL7 UV IPS (what Solum actually models) — [FHIR-VALIDATION.md](FHIR-VALIDATION.md)

## Probe procedure (repeatable)

1. Export Bundle: `./scripts/validate-fhir-ips.sh` (structural always; Java Validator optional).
2. Offline DE package (when available): point the same HL7 Validator at an ISiK/DE IG package instead of `hl7.fhir.uv.ips` and log errors into this dossier.
3. Optional network probe against a **public** validation or reference endpoint (only with operator credentials / ToS compliance) — record HTTP status + OperationOutcome; never commit secrets.

Record results under `examples/fhir-ips-export/out/` (gitignored) and summarise here after each campaign.

### Campaign log

| Date | Probe | Result |
|------|-------|--------|
| 2026-08-11 | Structural IPS export (`validate-fhir-ips.sh`) | **PASS** |
| 2026-08-11 | HL7 Validator 6.10.1 + IPS 2.0.0 (pre-harden) | **FAIL** — 7 errors / 5 warnings |
| 2026-08-11 | HL7 Validator 6.10.1 + IPS 2.0.0 (UUID/LOINC/ait-1/narrative) | **Success** — 0 errors / 0 warnings |
| 2026-08-11 | ISiK Basis Patient vs exported Patient | Gap — see table (`fail`) |
| — | Live gematik RU | Not run (requires operator access) |

## Gap → follow-up work (prioritised)

| Priority | Gap | Suggested follow-up |
|----------|-----|---------------------|
| P0 | ~~Crypto ignores active consent after revoke~~ **Done 2026-08-11** — `*_as` crypto requires grant covering category; see [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) | Issue [#1](https://github.com/SynapticFour/Solum/issues/1) |
| P1 | No ISiK Patient profile mapping | Pilot-gated mapper (see [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md)) |
| P1 | IPS emptyReason / terminology unbound | Align sections when DE pilot names target IG version |
| P2 | Composition metadata for DE document exchange | Add profile-specific Composition when pilot chooses document type |
| P2 | No TI auth integration | Remain out of Solum core; partner connector if ever required |
| P3 | Full IG conformance automation in CI | Add DE IG package pin + validator job only after P1 mapper exists |

Track these as GitHub issues:

1. [#1 proof: gate crypto encrypt/decrypt on active consent (Deny B)](https://github.com/SynapticFour/Solum/issues/1)
2. [#2 fhir: ISiK Basis Patient export/mapper spike (pilot-gated)](https://github.com/SynapticFour/Solum/issues/2)
3. [#3 fhir: bind IPS/ISiK section terminology for pilot IG version](https://github.com/SynapticFour/Solum/issues/3)
4. [#4 fhir: DE document Composition profile after pilot document-type choice](https://github.com/SynapticFour/Solum/issues/4)
5. [#5 ci: optional HL7 Validator job once DE/IPS package pin exists](https://github.com/SynapticFour/Solum/issues/5)
6. [#6 docs: record first gematik RU / public validator campaign in DE-FHIR-GAP](https://github.com/SynapticFour/Solum/issues/6)

## How this strengthens Solum without lying

Markets outside Germany still benefit: a written gap dossier shows you can speak DE FHIR ecosystems fluently. Inside Germany it is a **sales and engineering checklist**, not a certificate. Ship the [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) compliance proof first; use this file when a DE conversation starts.
