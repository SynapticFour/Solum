# Releasing Solum

This repository follows Semantic Versioning (`MAJOR.MINOR.PATCH`) for **GitHub Release tags** (`vX.Y.Z`). Stage-1 baseline tags (`stage1-baseline-*`) remain engineering freeze markers and are not SemVer product releases.

## Before the first production SemVer tag

1. Ensure [`.github/workflows/ci.yml`](.github/workflows/ci.yml) is green on `main`.
2. Ensure [`.github/workflows/release.yml`](.github/workflows/release.yml) has been exercised successfully. Prefer **Actions → Release → Run workflow** with `create_release=false` (dry-run artifacts) before the first `v*` tag.
3. Update [CHANGELOG.md](CHANGELOG.md): move `[Unreleased]` notes into a dated `## [X.Y.Z]` section.
4. Confirm workspace `version` in root [`Cargo.toml`](Cargo.toml) matches the intended tag (or document intentional drift).
5. **Do not** cut a production SemVer tag if release CI has never produced artifacts successfully.

## Cut a release

```bash
# On main, clean tree, CI green
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z   # only when you intend to publish
```

The `Release` workflow builds `solum` and `solum-sidecar` for:

| Asset | Platform |
|-------|----------|
| `solum-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 |
| `solum-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |
| `solum-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `solum-sidecar-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 sidecar |
| `solum-sidecar-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon sidecar |

and attaches them (plus `.sha256`) to the GitHub Release.

The same workflow also generates a **CycloneDX SBOM** (`solum-sbom.cdx.json`) and attaches it to the release.

## Operator install after a GitHub Release exists

See [docs/customer/DEPLOYMENT-RUNBOOK.md](docs/customer/DEPLOYMENT-RUNBOOK.md) §1 — prefer release assets when present; otherwise build from source at a baseline or SemVer tag.

## Versioning rules

- `MAJOR`: breaking CLI/API or custody behaviour changes
- `MINOR`: backward-compatible features
- `PATCH`: fixes and documentation

## Custody reminder

Paid evaluations and pilots must use **CustomerHeld `--keypair`** files (or library/KMS paths). Ephemeral keys require `SOLUM_ALLOW_EPHEMERAL=1` + `dev-local` (or another profile that allows `ephemeral_test`) and are **not** an evaluation path.
