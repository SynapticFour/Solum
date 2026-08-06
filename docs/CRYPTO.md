# Cryptography: Crypt4GH across Ferrum and Solum

Crypt4GH is a **universal envelope encryption scheme** (X25519 header packets + ChaCha20-Poly1305 payload segments). Synaptic Four uses it as the shared at-rest envelope for both genomic objects (Ferrum) and clinical field categories (Solum).

## Why Crypt4GH here (not a custom AEAD blob)

1. **One envelope across the portfolio** — same format, same key tooling, same threat model as Ferrum.
2. **No FHIR at-rest field standard** — HL7 FHIR security labels are metadata, not encryption. JWE (JOSE) appears in **SMART Health Links** and experimental bulk-export key delivery for *sharing* FHIR files; it is not a general “encrypt every PHI category at rest in a compliance layer” standard for EHDS Annex II–style deployments.
3. **Grain size is not a format veto** — Crypt4GH works for small payloads as well as BAM-scale streams; Ferrum’s DRS use case does not reserve the format for genomics alone.

## Layout

| Layer | Owner |
|-------|--------|
| Crypt4GH format + SynapticFour `crypt4gh` fork (no `rust-crypto`) | Vendored at [`third_party/crypt4gh`](../third_party/crypt4gh) (same lineage as Ferrum) |
| Genomic DRS encrypt / re-wrap / proxy | [Ferrum `ferrum-crypt4gh`](https://github.com/SynapticFour/Ferrum/blob/main/docs/CRYPT4GH.md) |
| Clinical category encrypt + custody policy | `solum-crypto` (`crypt4gh-v1` on [`EncryptedField`](../crates/crypto/src/lib.rs)) |

## Customer-held keys

Under `KeyCustody::CustomerHeld`, Solum does not mint keys *during encrypt* for regulated custody. Operators supply material via:

| Path | Use |
|------|-----|
| CLI `solum crypto keygen` + `--keypair` | Stage‑1 evaluation / pilot operator path (file-based CustomerHeld) |
| `CustomerHeldKeyProvider::register_customer_keypair` | Library integrators |
| Optional `AwsKmsKeyProvider` (`aws-kms` feature) | KMS-wrapped seeds; CLI `wrap-seed` / `--wrapped-keypair`; sidecar `--wrapped-keys-dir` |

`generate_operator_keypair` / `crypto keygen` produce bytes for the operator to persist and register — Solum does not retain them after write.

## Ephemeral / test keys (dev only)

`EphemeralTestKeyProvider` and CLI `--ephemeral` may mint keys **only** for local demos:

1. Set `SOLUM_ALLOW_EPHEMERAL=1`, and
2. Use a profile that lists `ephemeral_test` (e.g. `config/profiles/dev-local.toml`).

Pilot profiles (`eu-ehds`, `kenya-dpa`) allow **only** `customer_held` and refuse `EphemeralTest` at startup. **Paid evaluations must not use ephemeral keys.**

## SMART Health Links / JWE (out of scope for stage-1 at-rest)

If Solum later **exports** FHIR via SMART Health Links or similar share manifests, a JWE adapter can sit *beside* Crypt4GH at-rest storage. That is an interchange concern, not a reason to replace Crypt4GH for residency-bound clinical fields.

## Related

- [ferrum.md](ferrum.md)
- Ferrum: [CRYPT4GH.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/CRYPT4GH.md)
