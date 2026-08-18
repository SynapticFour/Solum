# Solum documentation

Solum is a **clinical-data compliance layer**: enforce, translate, and evidence conforming processing. Default **Track A** sits beside an existing EHR. Optional **Track B** can front an openEHR CDR (EHRbase) for partner APIs — not a hospital EHR UI. See [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) · [H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md) · Demo [COVERAGE](https://github.com/SynapticFour/Solum-Demo/blob/main/docs/COVERAGE.md).

## Contents

| Doc | Topic |
|-----|--------|
| [GETTING-STARTED.md](GETTING-STARTED.md) | Prove + CLI (Stage-1) |
| [architecture.md](architecture.md) | Tracks, crates, startup enforcement, honest zero-knowledge path |
| [FOR-EVALUATORS.md](FOR-EVALUATORS.md) | Maturity, license, tested vs not |
| [roadmap.md](roadmap.md) | Stage 1 vs stage 2 |
| [profiles.md](profiles.md) | Jurisdiction TOML schema and planned files |
| [ferrum.md](ferrum.md) | What is reused vs linked from Ferrum |
| [CRYPTO.md](CRYPTO.md) | Shared Crypt4GH envelope (Ferrum objects · Solum fields) |
| [helios.md](helios.md) | Evidence / HELIOS boundary (**signing deferred**) |
| [ECOSYSTEM.md](ECOSYSTEM.md) | Synaptic Four related projects (Solum · Ferrum · HELIOS · Lab Kit) |
| [CHANGELOG.md](../CHANGELOG.md) | Keep a Changelog |
| [RELEASING.md](../RELEASING.md) | SemVer / GitHub Release process |
| [LICENSE](../LICENSE) | BUSL-1.1 ([notes](LICENSE-OPTIONS.md)) |
| [LICENSE-COMPATIBILITY.md](../LICENSE-COMPATIBILITY.md) | Allowed dependency licenses |
| [COMPATIBILITY.md](COMPATIBILITY.md) | API / release compatibility & BUSL Change Date |
| [H3-EHRBASE-BACKUP.md](H3-EHRBASE-BACKUP.md) | Track B backup/restore procedure |
| [H3-CDR-BACKUP-DRILL.md](H3-CDR-BACKUP-DRILL.md) | Site sign-off checklist for CDR drills |

## Proof / evidence path

| Doc | Topic |
|-----|--------|
| [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) | **Master map:** allowed claim → demo/command → forbidden twin · `./scripts/demo-claims-proof.sh` · Solum-Demo mirror: `make smoke-*` |
| [PRIORITIES.md](PRIORITIES.md) | Living engineering priority list (post Validator Success) |
| [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) | Track A compliance worked example (consent, crypto Deny, audit) |
| [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) | Track B EHRbase smoke evidence packaging (Solum-Demo `make smoke-h3`) |
| [FHIR-VALIDATION.md](FHIR-VALIDATION.md) | IPS-oriented Bundle export vs FHIR Validator |
| [DE-FHIR-GAP.md](DE-FHIR-GAP.md) | German reference (ISiK / gematik) gap dossier — **not** a TI claim |
| [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md) | Narrow KIS adapter on main; full ISiK IG still **pilot-gated** |
| [AUTH-HOSPITAL.md](AUTH-HOSPITAL.md) | Klinik-OIDC / SMART Backend Services (not App Launch, not Passports) |

## Do not duplicate Ferrum docs

GA4GH services (Beacon, DRS, htsget, WES, TES, TRS), Passports, and Crypt4GH envelope details are documented upstream:

- [Ferrum README](https://github.com/SynapticFour/Ferrum/blob/main/README.md)
- [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md)
- [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit)

## Public-repo hygiene

Do not commit pricing, named sales partner shortlists, trademark candidate lists, or internal go-to-market checklists. Keep those outside this tree (see PRODUCT-DEFINITION §10).
