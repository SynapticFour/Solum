# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Phase C evaluation pack — CustomerHeld CLI crypto** — `solum crypto keygen` writes operator keypair files; encrypt/decrypt require `--keypair` for CustomerHeld custody (pilot / paid-evaluation path).
- **Ephemeral gate** — `--ephemeral` requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile that allows `ephemeral_test` (`config/profiles/dev-local.toml`). Pilot profiles (`eu-ehds`, `kenya-dpa`) refuse `EphemeralTest` custody at startup.
- **Release workflow** — `.github/workflows/release.yml` builds `solum` CLI tarballs on `v*` tags and attaches them to a GitHub Release (verify green before cutting a production SemVer tag; see [RELEASING.md](RELEASING.md)).
- **Release docs** — [CHANGELOG.md](CHANGELOG.md), [RELEASING.md](RELEASING.md); customer runbook documents from-source vs future GitHub Release assets.

### Changed

- Consent CLI paths use an empty `CustomerHeldKeyProvider` (no accidental ephemeral custody for pilot profiles).
- Customer docs ban ephemeral keys for paid-evaluation language; HELIOS live signing marked deferred / not productized ([docs/helios.md](docs/helios.md), [docs/roadmap.md](docs/roadmap.md)).

### Notes

- Kenya profile remains **DRAFT**. Stage-1 evaluation language unchanged.
- Do **not** cut a production `v*` tag until release CI binaries build successfully.

[Unreleased]: https://github.com/SynapticFour/Solum/compare/stage1-baseline-website-2026-07-30...HEAD
