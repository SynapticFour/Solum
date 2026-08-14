# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **Sidecar AuthZ** — pilot profiles (`eu-ehds`, `kenya-dpa`) require org-IAM (issuer + audience). Body `capability[]` is not an authorization source except `dev-local`. Ferrum `GET /v1/consent/status` on those profiles needs a Bearer JWT mapped to `solum:consent:read` (token alone is 403).
- **Object-bound consent** — FHIR GET, subject-link GET, composition GET, and AQL must bind to the consented subject (fail-closed IDOR).
- **Encryption at rest** — FHIR façade, subject-link, and dual-write dead-letter JSONL are Crypt4GH envelopes; leftover plaintext lines refuse to load.
- **Audit/consent GET** — `/v1/audit/export`, `/v1/audit/verify`, `/v1/consent/status` require the matching capability (no token-only export).
- **Listen policy** — non-loopback HTTP bind is refused; terminate TLS at a reverse proxy in front of `127.0.0.1`. `SOLUM_ALLOW_PLAINTEXT_HTTP=1` is honoured only with `dev-local` (Docker eval).
- **JWKS** — stale URL refresh failure is fail-closed (503), not silent reuse of old keys.
- **`link_cdr`** — refused (no EHRbase example compositions as patient data). CDR commit defaults `use_example=false`.
- **Residency attestation** — pilot CLI/sidecar require explicit `SOLUM_STORAGE_REGION`; EU/EEA attestation refuses a contradictory `AWS_REGION`.
- **JSONL budget** — façade files rotate at `SOLUM_JSONL_MAX_BYTES` (default 256 MiB). Audit hash-chain **refuses** appends above `SOLUM_AUDIT_MAX_BYTES` (default 512 MiB) instead of rotating evidence.

### Changed

- Kenya profile status **EVALUATION-ONLY** (was incorrectly labelled a production candidate). Docs no longer claim “EU and Africa as equal cores.”
- Org-IAM JWTs must carry `iss` and `aud`.
- CONTRIBUTING: no direct pushes to `main`; focused commits; no self-merge.
- CodeQL on pull_request; dependency-review is fail-closed.
- `deny.toml` default graph: `all-features = false`, `exclude-dev = true`.
- CI `feature-paths` job: `ferrum-storage-backend` and mocked `aws-kms` on rustc 1.94.1 (AWS SDK MSRV); vendored crypt4gh on 1.91.1.
- Release workflow: `workflow_dispatch` dry-run; builds `solum-sidecar` alongside `solum`.

### Fixed

- **H3 Demo Dockerfile** — `deploy/h3-ehrbase/Dockerfile.sidecar` drops local `.cargo/config.toml` Ferrum path-patch so `make up-h3` builds without a sibling mount.
- **IPS Bundle HL7 Validator** — deterministic UUID v5 `fullUrl`s, LOINC display **Patient Summary**, AllergyIntolerance `clinicalStatus` (ait-1), generated narratives → Validator + `hl7.fhir.uv.ips#2.0.0` **Success** (0 errors / 0 warnings).
- **Composition.author** — Organization Bundle entry + `reference` (closes display-only author ANNAHME).

### Changed

- **Roadmap / PRODUCT-DEFINITION** — Track B H3 engineering exit reflected; open gates pointed at Showcase [HORIZON-OPEN-GATES](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/HORIZON-OPEN-GATES.md).
- **Kenya / HELIOS / FHIR honesty** — SECURITY-OVERVIEW Kenya status aligned to provisional; HELIOS remains export-envelope only.
- **Legacy `&str` Deployment APIs** — `grant_consent` / `revoke_consent` / `encrypt_field` / `decrypt_field` marked `#[deprecated]` in favour of `*_as`.

### Added

- **Consent-gated crypto (Deny B)** — `encrypt_field_as` / `decrypt_field_as` require active consent covering the field category (`consent.denied`); CLI `--subject`/`--purpose`; sidecar JSON parity; worked example enforces post-revoke decrypt refusal ([WORKED-EXAMPLE.md](docs/WORKED-EXAMPLE.md)).
- **Proof path** — Track A [WORKED-EXAMPLE.md](docs/WORKED-EXAMPLE.md) + [`examples/compliance-worked-example/`](examples/compliance-worked-example/); `verify.sh` §8; Track B [H3-WORKED-EVIDENCE.md](docs/H3-WORKED-EVIDENCE.md); IPS export + [FHIR-VALIDATION.md](docs/FHIR-VALIDATION.md); DE dossier [DE-FHIR-GAP.md](docs/DE-FHIR-GAP.md); pilot-gated [DE-ADAPTER-SPIKE.md](docs/DE-ADAPTER-SPIKE.md).
- **Planned profile scaffolds** — Nigeria NDPA + South Africa POPIA under `config/profiles/planned/` (not auto-loaded; not counsel-reviewed).
- **Claims proof trail** — [CLAIMS-PROOF-TRAIL.md](docs/CLAIMS-PROOF-TRAIL.md) maps every allowed Stage‑1 claim to a demo/command; `./scripts/demo-claims-proof.sh` runs Track A + FHIR structural + Kenya fail-closed checks; [PRIORITIES.md](docs/PRIORITIES.md) living priority list.
- **KMS EncryptionContext** — new `wrap-seed` / unwrap paths bind `solum:purpose` + `solum:key_ref`; legacy empty-context files still load.
- **`solum fhir export-ips`** — thin CLI Bundle export; `Deployment::encrypt_patient_summary_as` / `decrypt_patient_summary_as` for audited typed crypto.
- **Passport SolumActor tests** — Jwt + Passport fixtures in `solum_actor_auth`.
- **Migration dry rehearsal** — `./scripts/migration-rehearsal-dry-run.sh`.
- **BASELINE honesty refresh** — corrects stale openEHR-scaffold / KMS-unwired freeze text against HEAD ([BASELINE.md](docs/BASELINE.md)).
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
