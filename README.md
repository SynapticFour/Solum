# Solum

[![CI](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL-1.1-blue.svg)](LICENSE)
[![Rust 1.91.1](https://img.shields.io/badge/rust-1.91.1-orange.svg)](rust-toolchain.toml)

Clinical-data compliance layer: policy, interchange (FHIR / openEHR), consent, audit, and field encryption. Default Track A sits beside an existing EHR. Optional Track B can front EHRbase — that is not a Synaptic Four hospital EHR UI.

**Maturity: Early access** (Stage-1 / evaluation). Shipping core profile: `eu-ehds.toml`. Kenya (`kenya-dpa.toml`) is **evaluation-only**. Nigeria / South Africa under `config/profiles/planned/` are draft scaffolds, not auto-loaded.

> This README describes technical capabilities, not legal advice. See [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md). Solum does not interpret clinical data for diagnosis or therapy.

These public repositories are maintained by the same organisation and are designed to work together. Each repository keeps its own version and license. For details on roles, maturity, and how the components relate to one another, see [SUITE-OVERVIEW](https://github.com/SynapticFour/.github/blob/main/profile/SUITE-OVERVIEW.md).

## Quick start

```bash
make prove
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

Requires Rust 1.91.1 and libsodium. A local interactive stack is [Solum-Demo](https://github.com/SynapticFour/Solum-Demo), not this repo.

## Documentation

- [Getting started](docs/GETTING-STARTED.md)
- [Architecture](docs/architecture.md)
- [For evaluators](docs/FOR-EVALUATORS.md)
- [Product definition](docs/PRODUCT-DEFINITION.md) · [Documentation index](docs/README.md)

## License

Business Source License 1.1 — see [LICENSE](LICENSE).
