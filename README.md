# Solum

[![CI](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/Solum/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue.svg)](LICENSE)
[![Rust 1.91.1](https://img.shields.io/badge/rust-1.91.1-orange.svg)](rust-toolchain.toml)

**Clinical-data compliance layer** for EU EHDS, with evaluation profiles for other jurisdictions.

Built by **[Synaptic Four](https://synapticfour.com)**. Ferrum for genomic data · **Solum for clinical data** · shared sovereignty philosophy — separate brand, repository, and regulatory boundary.

> **Legal notice:** This README describes technical capabilities, not legal advice. Compliance with EHDS, GDPR, or other frameworks depends on the operator’s legal basis, configuration, and organisational measures. See [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md).

## Synaptic Four portfolio

Solum is the clinical compliance sibling to [Ferrum](https://github.com/SynapticFour/Ferrum). **Who it is for:** [docs/IDENTITY.md](docs/IDENTITY.md). Related projects: **[docs/ECOSYSTEM.md](docs/ECOSYSTEM.md)**.

Solum’s **default (Track A)** is a compliance layer: policy, interchange (FHIR / openEHR), and evidence — wherever clinical data already lives. An **optional Track B** can front an openEHR CDR (EHRbase) for partner persistence APIs; that is **not** a Synaptic Four hospital EHR UI. See [PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md) and [H3-EHRBASE-SPIKE.md](docs/H3-EHRBASE-SPIKE.md).

Local interactive proofs: [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) (`make smoke-all` · [COVERAGE](https://github.com/SynapticFour/Solum-Demo/blob/main/docs/COVERAGE.md)).

| | Ferrum | Solum |
|---|--------|--------|
| Domain | Genomic / -omic | Clinical (EHR-oriented) |
| Role | GA4GH data platform | Compliance layer (enforce · translate · evidence) |
| Interop focus | [GA4GH](https://github.com/SynapticFour/Ferrum) (see Ferrum docs) | FHIR (stage 1), openEHR (stage 2) |
| Crypto | Crypt4GH for genomic DRS objects; customer-held keys | **Same Crypt4GH envelope** for clinical field categories + customer-held keys (`crates/crypto`); see [docs/CRYPTO.md](docs/CRYPTO.md) |

Working name **Solum**. **Shipping core:** EU EHDS (`eu-ehds.toml`). **Evaluation:** Kenya DPA (`kenya-dpa.toml`) — not counsel-reviewed, not a production candidate. **Draft scaffolds only:** Nigeria NDPA and South Africa POPIA under `config/profiles/planned/`. Egypt is not in this tree.

Solum does **not** re-document GA4GH. For Beacon, DRS, Passports, Crypt4GH, and genomic EHDS notes, read [Ferrum](https://github.com/SynapticFour/Ferrum) and [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md).

## Workspace

```
Solum/
  crates/
    core/         # product orchestration + `solum` CLI
    sidecar/      # HTTP façade for non-Rust HMIS/EHR integrators
    identity/     # SolumActor, CAP_*, require_capability
    auth-verify/  # JWT/JWKS verification (org-IAM)
    profiles/     # jurisdiction TOML profiles + startup validation
    fhir/         # FHIR adapter (stage 1)
    openehr/      # openEHR / EHRbase client (Track B, stage 3.1 FHIR+AQL)
    audit/        # audit events; persistent hash-chained log, HELIOS export prepared
    crypto/       # encryption / key custody; pins ferrum-core
    consent/      # consent & access-rights engine (grant/revoke, purpose binding)
  config/profiles/
  docs/
```

## Quick start

**Prerequisites:** Rust 1.91.1 (via [`rust-toolchain.toml`](rust-toolchain.toml) / rustup) and **libsodium** (e.g. `brew install libsodium` on macOS, `apt install libsodium-dev` on Linux) — required by Crypt4GH (`libsodium-sys`).

```bash
make prove
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

# Must fail (non-zero): profile requires EU/EEA residency
SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

`ferrum-core` is git-pinned like [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit) (`crates/crypto` + `config/ci/ferrum-revision.txt`). For a local Ferrum sibling checkout, see `.cargo/config.toml.example`.

## CLI usage

The `solum` binary (crate `solum-core`) exposes jurisdiction check plus Deployment-backed consent, crypto, and audit subcommands. Paths below are examples; use a dedicated working directory for stores.

```bash
PROFILE=config/profiles/eu-ehds.toml
AUDIT=/tmp/solum-demo/audit.jsonl
CONSENT=/tmp/solum-demo/consent.jsonl
mkdir -p /tmp/solum-demo
export SOLUM_STORAGE_REGION=EU

# 1. Profile / runtime conformance (operator must attest region)
cargo run -p solum-core -- check --profile "$PROFILE"

# 2–4. Consent
# --capability is required for mutating consent (GTM-1 fail-closed: omit → denied).
# --scope remains consent *data categories*, not authorization.
cargo run -p solum-core -- consent grant \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor practitioner/7 \
  --capability solum:consent:grant \
  --scope patient_summary
cargo run -p solum-core -- consent status \
  --profile "$PROFILE" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision
# → granted | revoked | unknown  (no --audit required; read-only)
cargo run -p solum-core -- consent revoke \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor patient/42 \
  --capability solum:consent:revoke

# 5–6. Field encrypt / decrypt (CustomerHeld — evaluation / pilot path)
echo 'demo-summary' > /tmp/solum-demo/plain.txt
cargo run -p solum-core -- crypto keygen \
  --key-ref customer/demo-1 --out /tmp/solum-demo/customer.keypair.json
cargo run -p solum-core -- crypto encrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --category patient_summary --key-ref customer/demo-1 \
  --keypair /tmp/solum-demo/customer.keypair.json \
  --actor practitioner/7 --capability solum:crypto:encrypt \
  --in /tmp/solum-demo/plain.txt --out /tmp/solum-demo/field.json
cargo run -p solum-core -- crypto decrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --key-ref customer/demo-1 \
  --keypair /tmp/solum-demo/customer.keypair.json \
  --actor practitioner/7 --capability solum:crypto:decrypt \
  --in /tmp/solum-demo/field.json --out /tmp/solum-demo/plain-out.txt

# 7–8. Audit (export envelope only — live HELIOS signing is not productized)
cargo run -p solum-core -- audit export --audit "$AUDIT" --out /tmp/solum-demo/helios.json
cargo run -p solum-core -- audit verify --audit "$AUDIT"
# → ok
```

> **CustomerHeld `--keypair` (evaluations / pilots)**
> Use `crypto keygen` + `--keypair` for Stage‑1 evaluations. Ephemeral keys (`--ephemeral`) require `SOLUM_ALLOW_EPHEMERAL=1` and `config/profiles/dev-local.toml` — pilot profiles refuse them. See [docs/customer/DEPLOYMENT-RUNBOOK.md](docs/customer/DEPLOYMENT-RUNBOOK.md) §4.


## Jurisdiction profiles

Initial profile: [`config/profiles/eu-ehds.toml`](config/profiles/eu-ehds.toml) (EHDS Annex II–oriented). Kenya is an **evaluation** profile (not production). Nigeria / South Africa are draft scaffolds — not auto-loaded. Startup **refuses** to run when runtime storage, key custody, audit, or consent contradicts the active profile.

## Regulatory boundary (MDCG)

Solum manages, encrypts, logs, and translates — it does **not** interpret clinical data for diagnosis or therapy support. Before any feature that might cross that line, read **[CONTRIBUTING.md](CONTRIBUTING.md)**. The MDCG question is mandatory on issues and PRs. Design intent is not a medical device; that is not a legal certification — see [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md).

## License

**Business Source License 1.1** (aligned with Ferrum / Ferrum Lab Kit): Additional Use Grant for non-commercial research and internal research use; Change License Apache-2.0 after four years. See [LICENSE](LICENSE). Decision notes: [docs/LICENSE-OPTIONS.md](docs/LICENSE-OPTIONS.md).

## Documentation

Start at [docs/README.md](docs/README.md) — product definition, architecture, roadmap, profiles, HELIOS, ecosystem.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) (includes the mandatory MDCG boundary question) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Security reports: [SECURITY.md](SECURITY.md).

```bash
pre-commit install   # optional; same hooks as Ferrum / Lab Kit
make check           # fmt + clippy + test
```

Dependency updates are **manual / reviewed**. Dependabot is not used (Synaptic Four org policy).

## Contact

[contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)

---

<div align="center">
Clinical-data compliance layer · companion to Ferrum · Synaptic Four
<br />
© 2026 Synaptic Four · Licensed under BUSL-1.1 · Free for non-commercial research under the Additional Use Grant
</div>
