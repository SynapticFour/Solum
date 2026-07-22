# License Compatibility

Solum is licensed under BUSL-1.1 (aligned with Ferrum / Ferrum Lab Kit). Dependencies must be compatible with that posture.

## Allowed dependency licenses

- MIT
- Apache-2.0
- BSD-2-Clause, BSD-3-Clause
- ISC
- Unicode-DFS-2016 / Unicode-3.0
- Zlib, OpenSSL, CC0-1.0
- BUSL-1.1 (workspace crates and git-pinned `ferrum-core`)

## Explicitly forbidden dependency licenses

GPL-2.0, GPL-3.0, AGPL-3.0, LGPL (all versions)

These are enforced via [`deny.toml`](deny.toml) (`cargo deny check licenses`).

## Allowed git sources

Only `https://github.com/SynapticFour/Ferrum.git` (pinned `ferrum-core`). Other git dependencies require an explicit policy update.

## Verification

```bash
cargo deny check licenses
cargo deny check sources
```
