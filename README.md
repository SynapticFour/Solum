# Solum

**Clinical-data compliance layer** for EU EHDS and African data-protection regimes.

Ferrum for genomic data · **Solum for clinical data** · shared sovereignty philosophy — separate brand, repository, and regulatory boundary.

Solum **does not own durable clinical storage**. It enforces policy, translates interchange formats (FHIR / openEHR), and produces evidence of conforming processing — wherever the data already lives.

| | Ferrum | Solum |
|---|--------|--------|
| Domain | Genomic / -omic | Clinical (EHR-oriented) |
| Role | GA4GH data platform | Compliance layer (enforce · translate · evidence) |
| Interop focus | [GA4GH](https://github.com/SynapticFour/Ferrum) (see Ferrum docs) | FHIR (stage 1), openEHR (stage 2) |
| Crypto posture | Crypt4GH-style, customer-held keys | Same philosophy via git-pinned `ferrum-core`; policy in `crates/crypto` |

Working title **Solum** — final brand name may change. Markets: **EU and Africa as equal cores**, modelled as jurisdiction profile data (not hard-coded special cases).

Solum does **not** re-document GA4GH. For Beacon, DRS, Passports, Crypt4GH, and genomic EHDS notes, read [Ferrum](https://github.com/SynapticFour/Ferrum) and [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md).

## Workspace

```
Solum/
  crates/
    core/       # product orchestration + `solum` CLI
    profiles/   # jurisdiction TOML profiles + startup validation
    fhir/       # FHIR adapter (stage 1)
    openehr/    # openEHR adapter (stage 2 scaffold)
    audit/      # audit events, HELIOS export prepared
    crypto/     # encryption / key custody; pins ferrum-core
  config/profiles/
  docs/
```

## Quick start

```bash
cargo test --workspace
cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

# Must fail (non-zero): profile requires EU/EEA residency
SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

`ferrum-core` is git-pinned like [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit) (`crates/crypto` + `config/ci/ferrum-revision.txt`). For a local Ferrum sibling checkout, see `.cargo/config.toml.example`.

## Jurisdiction profiles

Initial profile: [`config/profiles/eu-ehds.toml`](config/profiles/eu-ehds.toml) (EHDS Annex II–oriented). Further profiles (Kenya, Nigeria, South Africa, …) are data files — no code change required. Startup **refuses** to run when runtime storage, key custody, audit, or consent contradicts the active profile.

## Regulatory boundary (MDCG)

Solum manages, encrypts, logs, and translates — it does **not** interpret clinical data for diagnosis or therapy support. Before any feature that might cross that line, read **[CONTRIBUTING.md](CONTRIBUTING.md)**. The MDCG question is mandatory on issues and PRs. Design intent is not a medical device; that is not a legal certification — see [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md).

## License

**Business Source License 1.1** (aligned with Ferrum / Ferrum Lab Kit): Additional Use Grant for non-commercial research and internal research use; Change License Apache-2.0 after four years. See [LICENSE](LICENSE). Decision notes: [docs/LICENSE-OPTIONS.md](docs/LICENSE-OPTIONS.md).

## Documentation

Start at [docs/README.md](docs/README.md) — product definition, architecture, roadmap, profiles, HELIOS.
