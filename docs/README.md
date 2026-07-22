# Solum documentation

Solum is the **clinical-data compliance layer** in the Synaptic Four portfolio.

| Product | Domain |
|---------|--------|
| **[Ferrum](https://github.com/SynapticFour/Ferrum)** | Genomic / -omic data, GA4GH APIs, Crypt4GH-style sovereignty |
| **Solum** | Clinical data (FHIR / openEHR), jurisdiction profiles (EU EHDS + African regimes) |

Shared philosophy: customer-held keys, residency enforcement, auditable access — **separate brands, repos, and regulatory boundaries**.

## Contents

- [Architecture overview](architecture.md)
- [Ferrum relationship](ferrum.md) — what Solum reuses vs. what stays in Ferrum
- [Jurisdiction profiles](profiles.md)
- [License](../LICENSE) — BUSL-1.1 ([decision notes](LICENSE-OPTIONS.md))
- [Product definition](PRODUCT-DEFINITION.md) — placeholder until the full brief is published

## Do not duplicate Ferrum docs

GA4GH services (Beacon, DRS, htsget, WES, TES, TRS), Passports, and Crypt4GH envelope details are documented upstream:

- [Ferrum README](https://github.com/SynapticFour/Ferrum/blob/main/README.md)
- [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md) (incl. EHDS readiness for genomic infrastructure)
- [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit) (deploy/profile patterns for Ferrum)
