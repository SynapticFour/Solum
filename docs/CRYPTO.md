# Cryptography: Ferrum Crypt4GH vs Solum field envelopes

Solum and Ferrum share a **sovereignty philosophy** (customer-held control, open standards, ChaCha20-Poly1305–class AEAD). They do **not** share the same wire/container format for every encrypted byte.

## What Ferrum’s Crypt4GH is for

[Ferrum Crypt4GH](https://github.com/SynapticFour/Ferrum/blob/main/docs/CRYPT4GH.md) (`ferrum-crypt4gh`) encrypts **genomic file/stream objects** in DRS:

- Crypt4GH container: X25519 header + **ChaCha20-Poly1305** payload segments (64 KiB)
- Designed for BAM/VCF-scale objects and O(1) header re-wrap on download
- Lab Kit does **not** link `ferrum-crypt4gh`; it only toggles server-side encrypt on Ferrum

Ferrum’s compliance wording separates concerns roughly as: TLS in transit, general at-rest controls, **Crypt4GH for genomics** ([COMPLIANCE.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md)).

## What Solum encrypts

Solum encrypts **clinical field / category blobs** (FHIR elements, consent records, identifiers listed in jurisdiction `required_field_categories`) under a compact **envelope**:

| Layer | Role |
|-------|------|
| DEK | Random per `encrypt_field` call; encrypts plaintext with ChaCha20-Poly1305 |
| KEK | Customer-held (HSM/KMS); Solum only references it via [`KeyRef`](../crates/crypto/src/lib.rs) — never mints KEKs under `CustomerHeld` |
| Format | Serde-friendly `EncryptedField` (`chacha20poly1305-envelope-v1`), not a Crypt4GH file |

Implementation: `solum-crypto` (RustCrypto `chacha20poly1305`). Same AEAD family as Crypt4GH payloads; **not** the Crypt4GH container.

## Why Solum does not call Crypt4GH for fields

1. **Wrong grain size** — Crypt4GH is a segmented file format; clinical fields are small, often column/JSON-sized values.
2. **Wrong product boundary** — Crypt4GH in Ferrum is tied to DRS object storage and GA4GH genomic exchange; Solum must not re-host that stack.
3. **Same philosophy, separate format** — customer-held keys + AEAD; when genomic blobs appear, hand them to **Ferrum/DRS + Crypt4GH**, do not re-implement Crypt4GH here.

## When Solum might touch Crypt4GH later

Only as an **integration**, not a reimplementation: e.g. a clinical record references a Ferrum DRS ID for an encrypted genomic attachment. Field PHI stays in Solum envelopes; the genomic object stays Crypt4GH under Ferrum.

## Related

- [ferrum.md](ferrum.md) — dependency and ownership boundaries
- [architecture.md](architecture.md) — customer-held keys / honest ZK path
- Ferrum: [CRYPT4GH.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/CRYPT4GH.md)
