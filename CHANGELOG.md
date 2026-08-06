# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **H2 spine — zeroize** — best-effort `ZeroizeOnDrop` for CustomerHeld / AwsKms held Crypt4GH seeds.
- **ADR 0002 — CDR engine** — front EHRbase (Apache 2.0) as Track B default; Solum keeps compliance/façade/migration ([docs/adr/0002-cdr-engine-ehrbase.md](docs/adr/0002-cdr-engine-ehrbase.md)).
- **Kenya K1 counsel brief** — external-review package for DRAFT `kenya-dpa` retention/transfer/HDB/offline questions ([docs/counsel/KENYA-K1-BRIEF.md](docs/counsel/KENYA-K1-BRIEF.md)).
- **ADR 0001 + migration strangler** — optional openEHR clinical data plane (Track B) and wrap→mirror→prefer→cut-over path ([docs/adr/0001-openehr-cdr-and-migration.md](docs/adr/0001-openehr-cdr-and-migration.md), [docs/MIGRATION-STRANGLER.md](docs/MIGRATION-STRANGLER.md)).
- **Phase C evaluation pack — CustomerHeld CLI crypto** — `solum crypto keygen` writes operator keypair files; encrypt/decrypt require `--keypair` for CustomerHeld custody (pilot / paid-evaluation path).
- **Ephemeral gate** — `--ephemeral` requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile that allows `ephemeral_test` (`config/profiles/dev-local.toml`). Pilot profiles (`eu-ehds`, `kenya-dpa`) refuse `EphemeralTest` custody at startup.
- **Release workflow** — `.github/workflows/release.yml` builds `solum` CLI tarballs on `v*` tags and attaches them to a GitHub Release (verify green before cutting a production SemVer tag; see [RELEASING.md](RELEASING.md)).
- **Release docs** — [CHANGELOG.md](CHANGELOG.md), [RELEASING.md](RELEASING.md); customer runbook documents from-source vs future GitHub Release assets.

### Changed

- Consent CLI paths use an empty `CustomerHeldKeyProvider` (no accidental ephemeral custody for pilot profiles).
- Customer docs ban ephemeral keys for paid-evaluation language; HELIOS live signing marked deferred / not productized ([docs/helios.md](docs/helios.md), [docs/roadmap.md](docs/roadmap.md)).

### Notes

- Kenya profile remains **DRAFT**; counsel brief ready ([docs/counsel/KENYA-K1-BRIEF.md](docs/counsel/KENYA-K1-BRIEF.md)); portfolio H4 names Kenya as first non-EU pack ([Showcase H4 decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md)).
- Stage-1 evaluation language unchanged; Track B CDR is architecture-only (ADR 0001 + 0002) until H3 implementation.
- Do **not** cut a production `v*` tag until release CI binaries build successfully.

[Unreleased]: https://github.com/SynapticFour/Solum/compare/stage1-baseline-website-2026-07-30...HEAD
