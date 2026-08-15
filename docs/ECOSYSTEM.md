# Synaptic Four — related projects

**You are here:** [Solum](https://github.com/SynapticFour/Solum) — clinical-data compliance layer (FHIR / openEHR, jurisdiction profiles, evidence hooks).

Solum shares Synaptic Four’s sovereignty philosophy with Ferrum but is a **separate product, brand, and regulatory perimeter**. Portfolio map (four products, not a bundle SKU): [Ferrum docs/PORTFOLIO.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/PORTFOLIO.md).

## Related repositories

| Repository | Role | License | Relation to Solum |
|------------|------|---------|-------------------|
| **Solum** (this repo) | Clinical compliance layer (+ optional Track B openEHR CDR façade) | BUSL-1.1 | — |
| [Ferrum](https://github.com/SynapticFour/Ferrum) | GA4GH genomic data/compute plane | BUSL-1.1 | Sibling product; Solum pins `ferrum-core` |
| [Ferrum-Lab-Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit) | Ferrum deploy/profile on-ramp | BUSL-1.1 | Same profile/pin patterns; not a Solum dependency |
| [HELIOS](https://github.com/SynapticFour/HELIOS) | Signed, reproducible evidence tooling | Apache-2.0 | Solum prepares audit export for HELIOS-class evidence |
| [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) | Interactive + smokes: Stage-1 authz/audit/consent; optional H3 Track B | — | Stage-1 from pinned tag; H3 builds local `../Solum` — see Demo [COVERAGE](https://github.com/SynapticFour/Solum-Demo/blob/main/docs/COVERAGE.md) / `make smoke-all` |
| [SynapticFour-Showcase](https://github.com/SynapticFour/SynapticFour-Showcase) | Evidence pack / outreach — not a product | Apache-2.0 | Pins + scripts; not a SKU |

## Ownership boundaries

| Concern | Owner |
|---------|--------|
| Genomic GA4GH APIs / Crypt4GH platform | **Ferrum** |
| Ferrum lab deployment profiles | **Ferrum Lab Kit** |
| Clinical jurisdiction profiles, FHIR/openEHR compliance; optional CDR | **Solum** |
| Evidence packaging / signing mechanics | **HELIOS** (consumed by Solum; not vendored here) |
| Portfolio coordination / verification | **SynapticFour-Showcase** |

## Shared engineering conventions

Across Synaptic Four Rust products you should expect:

- BUSL-1.1 open-core parameters (where applicable) with research Additional Use Grant
- `cargo fmt` / `clippy -D warnings` / workspace tests
- `.pre-commit-config.yaml` (fmt + clippy + basic hygiene)
- Org CI: `ci.yml`, `secret-scan.yml`, `dependency-review.yml`, `codeql.yml` (no `quality-gate.yml` in this repo)
- **No Dependabot** (dependency updates are deliberate / reviewed)
- Contact: [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)

## Further reading in this repo

- [Product definition](PRODUCT-DEFINITION.md) (Track A sidecar + Track B CDR)
- [ADR 0001 — openEHR CDR + migration](adr/0001-openehr-cdr-and-migration.md)
- [ADR 0002 — CDR engine = EHRbase](adr/0002-cdr-engine-ehrbase.md)
- [ADR 0003 — subject bridge](adr/0003-subject-bridge.md)
- [Subject bridge operator runbook](solum-subject-bridge-runbook.md) (mirrors Ferrum)
- [Partner EHR API](customer/PARTNER-EHR-API.md)
- [HELIOS ingest recipe (upstream)](https://github.com/SynapticFour/HELIOS/blob/main/docs/solum-ingest.md)
- [Migration strangler](MIGRATION-STRANGLER.md) · [cut-over checklist](MIGRATION-CUTOVER-CHECKLIST.md)
- Kenya counsel package is **private** (not in this public tree). The in-tree Kenya profile remains evaluation-only until that review lands.
- Portfolio [coordinated roadmap](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/COORDINATED-PORTFOLIO-ROADMAP.md)
- [Ferrum relationship](ferrum.md)
- [HELIOS relationship](helios.md)
