# Relationship to Ferrum

**Ferrum for genomic data, Solum for clinical data — shared sovereignty philosophy, separate regulatory perimeter.**

## What Solum reuses

- `ferrum-core` as a **git-pinned** dependency (same pattern as [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit)): see `crates/crypto/Cargo.toml` and `config/ci/ferrum-revision.txt`.
- Sovereignty ideas: customer-controlled cryptography, residency awareness, auditable access.

## What Solum does **not** do

- Re-implement GA4GH APIs or copy Ferrum service crates.
- Patch Ferrum upstream for Solum-only needs — product-specific logic stays in `crates/crypto` (and siblings).
- Own the genomic compliance narrative; link to Ferrum docs instead.

## Upstream references

- Ferrum platform: <https://github.com/SynapticFour/Ferrum>
- Ferrum compliance / EHDS notes: <https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md>
- Lab Kit Ferrum pin workflow: <https://github.com/SynapticFour/Ferrum-Lab-Kit/blob/main/docs/FERRUM-INTEGRATION.md>
