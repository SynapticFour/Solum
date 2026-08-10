# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **H3 Demo Dockerfile** — `deploy/h3-ehrbase/Dockerfile.sidecar` drops local `.cargo/config.toml` Ferrum path-patch so `make up-h3` builds without a sibling mount.

### Changed

- **Roadmap / PRODUCT-DEFINITION** — Track B H3 engineering exit reflected; open gates pointed at Showcase [HORIZON-OPEN-GATES](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/HORIZON-OPEN-GATES.md).

### Added

- **H5 preparedness (optional)** — `SOLUM_TENANT_ID` audit stamp (metadata only); [H5-KEY-CUSTODY-MANAGED.md](docs/H5-KEY-CUSTODY-MANAGED.md) for managed single-tenant custody + TEE sketch.
- **H4 Kenya K2** — KE `validate_transfer` fail-closed tests; sidecar `kenya-dpa` refuse ephemeral / wrong region + CustomerHeld+KE start; [H4-OFFLINE-SYNC-POLICY.md](docs/H4-OFFLINE-SYNC-POLICY.md); `solum check` Kenya docs in [profiles.md](docs/profiles.md).
- **H3 depth harden** — live dual-write webhook `POST /v1/migrate/dual-write` (201 / 202+dead-letter); Patient FHIR → auto subject-link; [H3-CLINICAL-MODELLING.md](docs/H3-CLINICAL-MODELLING.md); [H3-EHRBASE-BACKUP.md](docs/H3-EHRBASE-BACKUP.md); H3-MDCG-SEND-CHECKLIST.md (removed from public tree).
- **H3.1–H3.6 Track B MVP slices** — AQL proxy + FHIR façade (`/v1/fhir/*`, `/v1/cdr/aql`); `solum migrate fhir-import` / `dual-write-stub` + [MIGRATION-CUTOVER-CHECKLIST.md](docs/MIGRATION-CUTOVER-CHECKLIST.md); [ADR 0003 subject bridge](docs/adr/0003-subject-bridge.md) + `/v1/cdr/subject-link`; [PARTNER-EHR-API.md](docs/customer/PARTNER-EHR-API.md); H3-MDCG-INTERNAL-REVIEW.md (removed from public tree).
- **H3.0 EHRbase Track B spike** — `VERSIONS` pins (`ehrbase:2.34.0`, `ehrbase-v2-postgres:16.2`); `solum-openehr` EHRbase REST client; sidecar `--ehrbase-url` + `/v1/cdr/*` façade; audit events `cdr.*`; Solum-Demo compose overlay; [docs/H3-EHRBASE-SPIKE.md](docs/H3-EHRBASE-SPIKE.md).
- **H2.2 Org CAP** — sidecar `--org-iam-config` + JWKS maps OIDC groups to `CAP_*` (body `capability[]` ignored); `config/org-iam/pilot-groups.toml`; `solum-identity` mapper + `solum-auth-verify` groups claims.
- **H2.1 Ferrum consumer** — document that Ferrum may poll `GET /v1/consent/status` for purpose-bound DRS/WES deny ([SIDECAR-INTEGRATION.md](docs/customer/SIDECAR-INTEGRATION.md), [ferrum.md](docs/ferrum.md)).
- **H2 spine — zeroize** — best-effort `ZeroizeOnDrop` for CustomerHeld / AwsKms held Crypt4GH seeds.
- **ADR 0002 — CDR engine** — front EHRbase (Apache 2.0) as Track B default; Solum keeps compliance/façade/migration ([docs/adr/0002-cdr-engine-ehrbase.md](docs/adr/0002-cdr-engine-ehrbase.md)).
- **H2.4 AWS KMS CLI/sidecar** — feature `aws-kms`: `crypto wrap-seed` / `--wrapped-keypair`; sidecar `--wrapped-keys-dir`; env credentials; rustc ≥ 1.94.1 for this feature; envelope honesty (not HSM).
- **Kenya K1 Vorprüfung applied** — non-counsel engineering review → profile **PROVISIONAL-PRODUCTION-CANDIDATE**; `optional_purposes`; honesty on retention/transfer/HDB (docs/counsel/KENYA-K1-VORPRUEFUNG.md (removed from public tree)). **Real counsel still required.**
- **Kenya K1 send checklist** — operator steps to package and email the counsel brief (docs/counsel/KENYA-K1-SEND-CHECKLIST.md (removed from public tree)).
- **Kenya K1 counsel brief** — external-review package for `kenya-dpa` retention/transfer/HDB/offline questions (docs/counsel/KENYA-K1-BRIEF.md (removed from public tree)).
- **ADR 0001 + migration strangler** — optional openEHR clinical data plane (Track B) and wrap→mirror→prefer→cut-over path ([docs/adr/0001-openehr-cdr-and-migration.md](docs/adr/0001-openehr-cdr-and-migration.md), [docs/MIGRATION-STRANGLER.md](docs/MIGRATION-STRANGLER.md)).
- **Phase C evaluation pack — CustomerHeld CLI crypto** — `solum crypto keygen` writes operator keypair files; encrypt/decrypt require `--keypair` for CustomerHeld custody (pilot / paid-evaluation path).
- **Ephemeral gate** — `--ephemeral` requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile that allows `ephemeral_test` (`config/profiles/dev-local.toml`). Pilot profiles (`eu-ehds`, `kenya-dpa`) refuse `EphemeralTest` custody at startup.
- **Release workflow** — `.github/workflows/release.yml` builds `solum` CLI tarballs on `v*` tags and attaches them to a GitHub Release (verify green before cutting a production SemVer tag; see [RELEASING.md](RELEASING.md)).
- **Release docs** — [CHANGELOG.md](CHANGELOG.md), [RELEASING.md](RELEASING.md); customer runbook documents from-source vs future GitHub Release assets.

### Changed

- Consent CLI paths use an empty `CustomerHeldKeyProvider` (no accidental ephemeral custody for pilot profiles).
- Customer docs ban ephemeral keys for paid-evaluation language; HELIOS live signing marked deferred / not productized ([docs/helios.md](docs/helios.md), [docs/roadmap.md](docs/roadmap.md)).

### Notes

- Kenya profile is **PROVISIONAL-PRODUCTION-CANDIDATE** after non-counsel Vorprüfung; send checklist (removed from public tree) + brief (removed from public tree) still required for real counsel; portfolio H4 names Kenya as first non-EU pack ([Showcase H4 decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md)).
- Stage-1 evaluation language unchanged for the frozen tag; **Track B H3 engineering exit** (CDR façade + MVP slices) is available post-baseline — open gates (counsel/OPT/MDR) remain in Showcase [HORIZON-OPEN-GATES](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/HORIZON-OPEN-GATES.md).
- Do **not** cut a production `v*` tag until release CI binaries build successfully.

[Unreleased]: https://github.com/SynapticFour/Solum/compare/stage1-baseline-website-2026-07-30...HEAD
