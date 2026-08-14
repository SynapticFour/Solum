# License Compatibility

Solum is licensed under BUSL-1.1 (aligned with Ferrum / Ferrum Lab Kit). Dependencies must be compatible with that posture.

## Allowed dependency licenses

- MIT
- Apache-2.0
- BSD-2-Clause, BSD-3-Clause
- ISC
- Unicode-DFS-2016 / Unicode-3.0
- Zlib, OpenSSL, CC0-1.0
- CDLA-Permissive-2.0 (transitive webpki-roots via ferrum-core / reqwest)
- BUSL-1.1 (workspace crates and git-pinned `ferrum-core`)

## Explicitly forbidden dependency licenses

GPL-2.0, GPL-3.0, AGPL-3.0, LGPL (all versions)

These are enforced via [`deny.toml`](deny.toml) (`cargo deny check licenses`).

## Allowed git sources

Only `https://github.com/SynapticFour/Ferrum.git` (pinned `ferrum-core`). Other git dependencies require an explicit policy update.

## Accepted advisories

Explicit `cargo deny` ignore entries (see [`deny.toml`](deny.toml) `[advisories].ignore`).
Each row is an accepted-risk record — not a silent suppress.

| Advisory | Crate | Accepted | Reason | Revisit when |
|----------|-------|----------|--------|--------------|
| [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) (Marvin Attack: RSA timing sidechannel) | `rsa` 0.9.x | 2026-07-25 | Same advisory via two `jsonwebtoken` paths: (1) transitive `rsa` ← `jsonwebtoken` ← git-pinned `ferrum-core` ← `solum-crypto` / `solum-core` / `solum-profiles` (Ferrum shared RSA JWT types); (2) direct `jsonwebtoken` dependency of `solum-auth-verify` (`rust_crypto` feature, RS256 path). Not Solum’s Crypt4GH field encryption path (`crypt4gh` / `sodiumoxide` / `libsodium-sys`). No new risk — one advisory ID, two reference sources. No safe upgrade available upstream yet. | `ferrum-core` migrates off RSA-based JWT, or a patched `rsa` crate ships; then drop the ignore and re-run `cargo deny check advisories`. |
| [RUSTSEC-2026-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104) (CRL parse panic before signature verify) | `rustls-webpki` 0.101.7 | 2026-08-14 | Transitive via `rustls` 0.21.12 ← `aws-smithy-http-client` / AWS SDK KMS+S3+STS (`solum-crypto`, `ferrum-storage`). Patched `rustls-webpki` 0.103.13 is already in the lockfile on the `rustls` 0.23 path; `rustls` 0.21 cannot take 0.103. Panic is only reachable while parsing a CRL, before signature verification, on the AWS TLS stack — not Solum’s Crypt4GH path. | `aws-smithy-http-client` / `aws-sdk-*` drop `rustls` 0.21; then drop the ignore and re-run `cargo deny check advisories`. |
| [RUSTSEC-2026-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098) (URI name constraints incorrectly accepted) | `rustls-webpki` 0.101.7 | 2026-08-14 | Same unpatchable `rustls` 0.21.12 AWS SDK path as RUSTSEC-2026-0104. Fix is `rustls-webpki` ≥0.103.12. | Same revisit as RUSTSEC-2026-0104. |
| [RUSTSEC-2026-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099) (wildcard name constraints incorrectly accepted) | `rustls-webpki` 0.101.7 | 2026-08-14 | Same unpatchable `rustls` 0.21.12 AWS SDK path as RUSTSEC-2026-0104. Fix is `rustls-webpki` ≥0.103.12. | Same revisit as RUSTSEC-2026-0104. |

Analogous posture to the SynapticFour `crypt4gh` fork replacing unmaintained `rust-crypto` ([RUSTSEC-2022-0011](https://rustsec.org/advisories/RUSTSEC-2022-0011)): document the blast radius, keep the ignore narrow, and track upstream rather than vendoring a one-off JWT stack in Solum.

## Verification

```bash
cargo deny check licenses
cargo deny check sources
cargo deny check advisories
```
