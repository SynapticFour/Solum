# Releasing Solum

This repository follows Semantic Versioning (`MAJOR.MINOR.PATCH`) for **GitHub Release tags** (`vX.Y.Z`). Stage-1 baseline tags (`stage1-baseline-*`) remain engineering freeze markers and are not SemVer product releases.

## Before the first production SemVer tag

1. Run `make verify-release` on the commit you will tag. That is [`scripts/verify.sh`](scripts/verify.sh): fmt, clippy, tests, cargo-deny when installed, and the feature-path checks when Rust 1.94.1 is installed. Product CI does not run on push to `main` or on pull requests. Dispatch [`.github/workflows/ci.yml`](.github/workflows/ci.yml) or CodeQL from [docs/CI.md](docs/CI.md) when you want those jobs on a runner.
2. Ensure [`.github/workflows/release.yml`](.github/workflows/release.yml) has been exercised successfully. Prefer **Actions → Release → Run workflow** with `create_release=false` (dry-run artifacts) before the first `v*` tag.
3. Update [CHANGELOG.md](CHANGELOG.md): move `[Unreleased]` notes into a dated `## [X.Y.Z]` section.
4. Confirm workspace `version` in root [`Cargo.toml`](Cargo.toml) matches the intended tag (or document intentional drift).
5. **Do not** cut a production SemVer tag if release CI has never produced artifacts successfully.

## Cut a release

```bash
# Clean tree, make verify-release already passed
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z   # only when you intend to publish
```

## Release train (portfolio)

When **Ferrum** is tagged: bump `FERRUM_REF` in [VERSIONS.lock](VERSIONS.lock) and `config/ci/ferrum-revision.txt` the same week if the crypto pin moves. Showcase `Solum-tag` and Solum-Demo `Solum-ref` follow **this** repo’s SemVer tag. See [Ferrum PORTFOLIO.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/PORTFOLIO.md).

The `Release` workflow builds `solum` and `solum-sidecar` for:

| Asset | Platform |
|-------|----------|
| `solum-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 |
| `solum-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |
| `solum-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `solum-sidecar-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 sidecar |
| `solum-sidecar-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon sidecar |

and attaches them (plus `.sha256`) to the GitHub Release.

The same workflow generates two **CycloneDX** files with `cargo-cyclonedx` 0.5.9 (`cargo cyclonedx --format json`) and attaches them to the release:

| File | Crate | Binary |
|------|--------|--------|
| `solum-core.cdx.json` | `solum-core` | `solum` |
| `solum-sidecar.cdx.json` | `solum-sidecar` | `solum-sidecar` |

0.5.9 writes one `<package>.cdx.json` beside each workspace member's `Cargo.toml`. It has no `--workspace` flag and no `--output-file` flag, so it does not write a single workspace-wide file. The workspace sets `publish = false`, so no crate is published to crates.io. The release keeps the two crates whose binaries are the release assets.

The `github-release` job runs only when `build-binaries` and `sbom` both succeed, and it refuses to upload if either SBOM file is missing. A failed SBOM job does not create a GitHub Release.

## Publish a release for a tag that already exists

A `v*` push runs the workflow file stored on that tag. A later commit on `main` does not change the file the tag runs. Do not move the tag, and do not re-run that tag's workflow to pick up a workflow fix.

When the binary jobs uploaded artifacts and the release job did not create a release, publish the release against the existing tag:

1. Check out that tag. Generate the SBOM there with the command in [`.github/workflows/release.yml`](.github/workflows/release.yml): `cargo install cargo-cyclonedx --version 0.5.9 --locked`, then `cargo cyclonedx --format json`, then copy `crates/core/solum-core.cdx.json` and `crates/sidecar/solum-sidecar.cdx.json` and hash them.
2. Download the binary artifacts from the Actions run (`gh run download <run-id> --repo SynapticFour/Solum`).
3. Write `SHA256SUMS.txt` with `shasum -a 256` over the `.tar.gz` files.
4. Create the release with `gh release create <tag> --verify-tag`. `--verify-tag` aborts when the tag is absent, so the command does not create a new tag from `main`. Notes are the changelog section for that version.

For `v0.1.1`, run `37175987701` uploaded the five binary artifacts (not expired; they expire 2027-01-02). The `sbom` job failed and `github-release` was skipped, so there is no GitHub Release yet. From a checkout of tag `v0.1.1`:

```bash
cargo install cargo-cyclonedx --version 0.5.9 --locked
cargo cyclonedx --format json
cp crates/core/solum-core.cdx.json solum-core.cdx.json
cp crates/sidecar/solum-sidecar.cdx.json solum-sidecar.cdx.json
shasum -a 256 solum-core.cdx.json > solum-core.cdx.json.sha256
shasum -a 256 solum-sidecar.cdx.json > solum-sidecar.cdx.json.sha256

gh run download 37175987701 --repo SynapticFour/Solum --dir solum-v0.1.1-assets
shasum -a 256 $(find solum-v0.1.1-assets -name '*.tar.gz' | sort) > SHA256SUMS.txt
awk '/^## \[0.1.1\]/{p=1} /^\[Unreleased\]:/{p=0} p' CHANGELOG.md > solum-v0.1.1-notes.md

gh release create v0.1.1 \
  --repo SynapticFour/Solum \
  --verify-tag \
  --title 'v0.1.1' \
  --notes-file solum-v0.1.1-notes.md \
  solum-v0.1.1-assets/*/*.tar.gz \
  solum-v0.1.1-assets/*/*.tar.gz.sha256 \
  SHA256SUMS.txt \
  solum-core.cdx.json \
  solum-core.cdx.json.sha256 \
  solum-sidecar.cdx.json \
  solum-sidecar.cdx.json.sha256
```

`--verify-tag` aborts when `v0.1.1` is missing, so this does not create a tag. Do not pass `--target`.

## Operator install after a GitHub Release exists

See [docs/customer/DEPLOYMENT-RUNBOOK.md](docs/customer/DEPLOYMENT-RUNBOOK.md) §1 — prefer release assets when present; otherwise build from source at a baseline or SemVer tag.

## Versioning rules

- `MAJOR`: breaking CLI/API or custody behaviour changes
- `MINOR`: backward-compatible features
- `PATCH`: fixes and documentation

## Custody reminder

Paid evaluations and pilots must use **CustomerHeld `--keypair`** files (or library/KMS paths). Ephemeral keys require `SOLUM_ALLOW_EPHEMERAL=1` + `dev-local` (or another profile that allows `ephemeral_test`) and are **not** an evaluation path.
