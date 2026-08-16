# Solum purposes vs GA4GH DUO / ADS

Solum **purpose** strings are **jurisdiction-profile local** (EHDS, Kenya DPA, …). They are the consent/capability axis for clinical processing.

[ga4gh-infra](https://github.com/SynapticFour/ga4gh-infra) **DUO codes** and **ADS** decisions authorize *research dataset* access (Passports / visas). That is a different plane.

Do not treat a Solum `purpose=research` grant as a DUO visa, and do not treat `DUO:0000006` as hospital care consent. Glue is optional crosswalk, not a shared enum.

## EU EHDS profile (`config/profiles/eu-ehds.toml`)

| Solum purpose | Role | DUO/ADS |
|---------------|------|---------|
| `care_provision` | Primary use | None — clinical, not a research data-use ontology |
| `emergency_access` | Primary use (break-glass) | None |
| `quality_improvement` | Secondary, still clinical ops | None |
| `secondary_use_hdab` | EHDS HDAB-mediated secondary use | Nearest research code is often `DUO:0000006` (GRU/HMB); **not equivalent**. HDAB is a legal gateway, not a Passport visa |
| `research` | Appears on some profiles / BRA default for subject-link | `DUO:0000006` as a *label* only if the operator also has a Passport/ADS permit for the genomic objects |

BRA's default `SOLUM_SUBJECT_PURPOSE=research` is a subject-link convenience. Ferrum `[solum]` still fail-closes on `GET /v1/consent/status` for the purpose stored on the DRS object.

## Operator rule

1. Clinical bytes: Solum grant for `(subject, purpose)` from the active profile.
2. Genomic research bytes: Ferrum + Passport/DUO/ADS from ga4gh-infra (or Ferrum built-in passports).
3. Same human, both worlds: shared `solum_subject` string — two decisions, one identifier.
