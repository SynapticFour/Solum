# Solum

Solum records consent, checks whether access to a field is allowed, and writes an audit log. It runs as its own program beside the practice, clinic, or health-information system a manufacturer already ships.

For a manufacturer of that software.

**Status: Early use.** Stage 1, evaluation. Registry: SOLUM-MATURITY-001. This status is not a production hospital deployment and not a certificate.

[![CI](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL-1.1-blue.svg)](LICENSE)
[![Rust 1.91.1](https://img.shields.io/badge/rust-1.91.1-orange.svg)](rust-toolchain.toml)

GitHub may label this Other. The SPDX identifier is BUSL-1.1.

## What it does

- It runs as an HTTP service beside the existing system, so any programming language can call it. FHIR, a format for exchanging health records, is the primary exchange format for Stage 1. Registry: SOLUM-SIDECAR-001.
- An authorised person records consent for a person and a purpose, or withdraws it. The operator's sign-in system decides who may call these functions. Access to a field succeeds only when that consent covers the person, the purpose, and the field. Consent and access are logged in the same call. Registry: SOLUM-FLOW-001.
- On the pilot profiles, authorisation is a JWT, a signed token from the organisation's identity system. A capability list in the request body is ignored on those profiles. If the configuration contradicts the profile, Solum refuses to start. Registry: SOLUM-SIDECAR-001.
- Fields can be encrypted with keys the customer holds.
- The default track sits beside the existing system and does not keep the clinical record. An optional second track can front EHRbase, a server for openEHR records, when the operator points Solum at that store. openEHR is a model for those records.

## Limits

- Stage 1. Not production-tested. Not a hospital record system. Solum does not interpret findings and does not replace a certificate. Registry: SOLUM-MATURITY-001, SOLUM-USE-001, SOLUM-SIDECAR-001.
- The Kenya profile (`config/profiles/kenya-dpa.toml`) is for evaluation only. The Nigeria and South Africa files under `config/profiles/planned/` are draft scaffolds. They are not loaded with the shipping profiles. Registry: SOLUM-MATURITY-001.
- Solum does not decide the consent. A failed log write does not let the call succeed. The log is a hash chain: a later edit of the file shows up when the chain is checked. The chain is not a signature. Signing an export is [HELIOS](https://github.com/SynapticFour/HELIOS), a separate program, and Solum does not call it. A status read writes no log line. On a grant, the consent line is stored before the log line. Older library functions without the capability check are not the HTTP path. Registry: SOLUM-ACCESS-001, SOLUM-FLOW-001.
- Consent and the log sit in files whose paths the operator sets (`--audit`, `--consent-store`). The region is the operator's statement at startup (`SOLUM_STORAGE_REGION`). That statement is not proof of where the machine is. The operator can point the program at another store, at the optional clinical store, or at a key service. This page does not say that data never leaves the premises. Registry: SOLUM-DATA-LOCATION-001.
- The getting-started guide, the integration guide, and the operator handbook describe setup. They do not state support terms, and they do not say that Synaptic Four operates the system. Registry: SOLUM-HANDOVER-001.
- [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) uses the `dev-local` profile: ephemeral keys and a capability list sent by the caller. That posture is not a customer evaluation. Pilot profiles refuse it.

> This README describes technical capabilities, not legal advice. See [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md).

## Quick start

Prerequisites: Rust 1.91.1, pinned in `rust-toolchain.toml`, and libsodium. On macOS that library is `brew install libsodium`. On Debian it is the `libsodium-dev` package.

```bash
make prove
```

You should see cargo print `test result: ok` for the workspace suites, and at the end the line `Solum prove OK. Maintainer: make verify. Interactive: Solum-Demo make up`.

```bash
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

You should see `ok: profile 'eu-ehds' (jurisdiction EU) matches runtime configuration`.

`SOLUM_STORAGE_REGION=us-east-1` with the same profile exits non-zero and prints `fatal:` and the error. These commands do not leave a server running.

A longer path that needs another repository: [Solum-Demo](https://github.com/SynapticFour/Solum-Demo), then `make up` and `make smoke-ci`. That path needs Docker. It follows Solum tag v0.1.0, which is not this repository's default branch.

## Documentation

Every file under `docs/` is listed in [docs/README.md](docs/README.md). The groups are a tutorial, how-to guides, reference, and explanation.

### Tutorial

- [Getting started](docs/GETTING-STARTED.md)

### How-to guides

- [Sidecar integration](docs/customer/SIDECAR-INTEGRATION.md) — call the HTTP service from the manufacturer's software
- [Deployment runbook](docs/customer/DEPLOYMENT-RUNBOOK.md)
- [For manufacturers](docs/FOR-MANUFACTURERS.md) — documents for a technical file
- [Incident response](docs/INCIDENT_RESPONSE.md)
- Further task pages are in the [index](docs/README.md#how-to-guides)

### Reference

- [Security overview](docs/customer/SECURITY-OVERVIEW.md)
- [Threat model](docs/THREAT_MODEL.md)
- [For evaluators](docs/FOR-EVALUATORS.md)
- [Changelog](CHANGELOG.md)
- [Versions pins](VERSIONS)
- [Releasing](RELEASING.md)
- Further reference pages are in the [index](docs/README.md#reference)

### Explanation

- [Architecture](docs/ARCHITECTURE.md)
- [Product definition](docs/PRODUCT-DEFINITION.md)
- [Architecture decisions](docs/adr/0001-openehr-cdr-and-migration.md) — four records; the index lists each one
- [Relationship to HELIOS](docs/helios.md) — export only; signing stays in HELIOS

## Licence

Solum is licensed under the Business Source License 1.1. You can read the source. It is not an open-source licence. Each version becomes Apache-2.0 after four years. Non-commercial research, academic use and education are free to use, and so is an institution's internal research when that research is not a commercial service to third parties. Registry: SOLUM-LICENSE-001. Commercial use, embedding in a product that is sold, and a hosted service need a written licence. Not legal advice. The file is [LICENSE](LICENSE).

## Cite

Use [CITATION.cff](CITATION.cff). Prefer that file over a hand-copied line.

## Website

- Product page: [https://synapticfour.com/en/solum/](https://synapticfour.com/en/solum/)
- Technical profile: [https://synapticfour.com/en/technical/solum/](https://synapticfour.com/en/technical/solum/)

The same pages exist in German (`/de/solum/`, `/de/technik/solum/`) and in French (`/fr/solum/`, `/fr/technique/solum/`).

The map of repositories: [profile/README.md](https://github.com/SynapticFour/.github/blob/main/profile/README.md).

Contact: [contact@synapticfour.com](mailto:contact@synapticfour.com)
