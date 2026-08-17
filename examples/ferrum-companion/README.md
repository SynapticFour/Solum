# Ferrum-companion reference deployment (Mode B)

This example is **Modus B** from [`docs/INTEGRATION-ROADMAP.md`](../../docs/INTEGRATION-ROADMAP.md):

| Modus | Storage | Auth | Ferrum-Abhängigkeit |
|---|---|---|---|
| **B. Ferrum-Companion** | Kunde/`ferrum-storage` (optional) | `ferrum-core::auth` (optional) | git-gepinnt, wie heute bei Crypto |

## What it proves

1. **Shared Crypt4GH format** — A “Ferrum-side” encrypt path (direct `crypt4gh` crate, same library Ferrum genomic objects use) and Solum’s `solum_crypto::encrypt_field` for `patient_summary` share one keypair. Solum can decrypt the Ferrum-path ciphertext; raw `crypt4gh` can decrypt the Solum ciphertext.
2. **`ferrum_core::auth::AuthClaims` is usable** — constructs a Jwt-variant claims value via the re-export from `solum-crypto` (no token verification; Sprint 5).

## Important API note (inspected at pin `f28f2780…`, Ferrum v0.3.1)

`ferrum-core` does **not** expose Crypt4GH encrypt/decrypt helpers. Solum’s `crates/crypto` already uses the shared `crypt4gh` crate directly and only links `ferrum-core` for shared types. This example mirrors that split rather than inventing a parallel Ferrum crypto API.

Run from the repository root:

```bash
cargo run -p solum-example-ferrum-companion
```

`verify.sh` section 7 invokes the same command.
