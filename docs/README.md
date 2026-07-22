# Solum documentation

Solum is the **clinical-data compliance layer** in the Synaptic Four portfolio: enforce, translate, and evidence conforming processing — not a durable clinical store, and not a Ferrum fork.

| Product | Domain |
|---------|--------|
| **[Ferrum](https://github.com/SynapticFour/Ferrum)** | Genomic / -omic data, GA4GH APIs, Crypt4GH-style sovereignty |
| **Solum** | Clinical data (FHIR / openEHR), jurisdiction profiles (EU EHDS + African regimes) |

Shared philosophy: customer-held keys, residency enforcement, auditable access — **separate brands, repos, and regulatory boundaries**.

## Contents

| Doc | Topic |
|-----|--------|
| [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) | Positioning, markets, MDCG boundary, standards, partner model (public) |
| [architecture.md](architecture.md) | Principles, crates, startup enforcement |
| [roadmap.md](roadmap.md) | Stage 1 vs stage 2 |
| [profiles.md](profiles.md) | Jurisdiction TOML schema and planned files |
| [ferrum.md](ferrum.md) | What is reused vs linked from Ferrum |
| [helios.md](helios.md) | Evidence / HELIOS boundary |
| [LICENSE](../LICENSE) | BUSL-1.1 ([notes](LICENSE-OPTIONS.md)) |

## Do not duplicate Ferrum docs

GA4GH services (Beacon, DRS, htsget, WES, TES, TRS), Passports, and Crypt4GH envelope details are documented upstream:

- [Ferrum README](https://github.com/SynapticFour/Ferrum/blob/main/README.md)
- [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md)
- [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit)

## Public-repo hygiene

Do not commit pricing, named sales partner shortlists, trademark candidate lists, or internal go-to-market checklists. Keep those outside this tree (see PRODUCT-DEFINITION §10).
