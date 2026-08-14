# Relationship to Ferrum

**Ferrum for genomic data, Solum for clinical data — shared sovereignty philosophy, separate regulatory perimeter.**

Ferrum is the GA4GH-oriented genomic platform. Solum is a clinical **compliance layer** (policy, translation, evidence) and does not replace Ferrum or re-host GA4GH APIs. Product framing: [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md).

## What Solum reuses

- `ferrum-core` as a **git-pinned** dependency (same pattern as [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit)): see `crates/crypto/Cargo.toml` and `config/ci/ferrum-revision.txt`. Bump with `./scripts/bump-ferrum.sh`.
- Sovereignty ideas: customer-controlled cryptography, residency awareness, auditable access.
- The **Crypt4GH envelope format** for clinical field categories (vendored SynapticFour `crypt4gh` fork, same as Ferrum) — see [CRYPTO.md](CRYPTO.md).

## What Solum does **not** do

- Re-implement GA4GH APIs or copy Ferrum DRS/Beacon/gateway crates.
- Patch Ferrum upstream for Solum-only needs — product-specific orchestration stays in this repo.
- Own the genomic compliance narrative; link to Ferrum docs instead.
- Become a durable clinical data lake — storage remains with the operator’s systems of record.
- Re-host Ferrum’s Crypt4GH *proxy* / DRS re-wrap service (Solum uses the format; Ferrum owns genomic object delivery).

## Consent status → Ferrum (H2.1)

Ferrum may poll `GET /v1/consent/status` to enforce purpose-bound deny on DRS/WES when `[solum]` is enabled. The sidecar token is required. On **pilot profiles** Ferrum must also send a Bearer JWT whose groups map to `solum:consent:read` (token alone is refused). Solum remains the system of record for grant/revoke; Ferrum does not store consent. Contract: Showcase [ADR 0001](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/adr/0001-solum-ferrum-consent-access.md).

## Subject bridge (H3.3)

Canonical join key `solum_subject_id` must match Ferrum DRS/WES metadata `solum_subject`. Operator steps: [solum-subject-bridge-runbook.md](solum-subject-bridge-runbook.md) · [ADR 0003](adr/0003-subject-bridge.md).

## Upstream references

- Ferrum platform: <https://github.com/SynapticFour/Ferrum>
- Ferrum compliance / EHDS notes: <https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md>
- Lab Kit Ferrum pin workflow: <https://github.com/SynapticFour/Ferrum-Lab-Kit/blob/main/docs/FERRUM-INTEGRATION.md>
