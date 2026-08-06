# Synaptic Four — related projects

**You are here:** [Solum](https://github.com/SynapticFour/Solum) — clinical-data compliance layer (FHIR / openEHR, jurisdiction profiles, evidence hooks).

Solum shares Synaptic Four’s sovereignty philosophy with Ferrum but is a **separate brand, repository, and regulatory perimeter**. For the five-repo **GA4GH stack** (identity, gateway, lab deploy, demo, conformance), see Ferrum’s mirrored map: [Ferrum docs/ECOSYSTEM.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/ECOSYSTEM.md).

## Related repositories

| Repository | Role | License | Relation to Solum |
|------------|------|---------|-------------------|
| **Solum** (this repo) | Clinical compliance layer (+ planned optional openEHR data plane) | BUSL-1.1 | — |
| [Ferrum](https://github.com/SynapticFour/Ferrum) | GA4GH genomic data/compute plane | BUSL-1.1 | Sibling product; Solum pins `ferrum-core` |
| [Ferrum-Lab-Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit) | Ferrum deploy/profile on-ramp | BUSL-1.1 | Same profile/pin patterns; not a Solum dependency |
| [HELIOS](https://github.com/SynapticFour/HELIOS) | Signed, reproducible evidence tooling | Apache-2.0 | Solum prepares audit export for HELIOS-class evidence |
| [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) | Local Stage-1 demo (fail-closed authz + tamper-evident audit) | — | Consumes pinned Solum tags; does not modify Solum |
| [SynapticFour-Showcase](https://github.com/SynapticFour/SynapticFour-Showcase) | Multi-repo evidence-chain integrator | Apache-2.0 | Orchestrates Ferrum + HELIOS + BRA + Solum stages — see Showcase coordinated roadmap |

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
- Org CI: `ci.yml`, `quality-gate.yml`, `secret-scan.yml`, `dependency-review.yml`, `codeql.yml`
- **No Dependabot** (dependency updates are deliberate / reviewed)
- Contact: [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)

## Further reading in this repo

- [Product definition](PRODUCT-DEFINITION.md) (Track A sidecar + Track B CDR)
- Portfolio [coordinated roadmap](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/COORDINATED-PORTFOLIO-ROADMAP.md)
- [Ferrum relationship](ferrum.md)
- [HELIOS relationship](helios.md)
