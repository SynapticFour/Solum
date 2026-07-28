# Stage 1 baseline (GTM-1 capability-based authorization)

| | |
|---|---|
| **Date** | 2026-07-28 |
| **Verified commit** | `3d822b7ec52acae348115efd8a56a86f8f14941d` |
| **Tag** | `stage1-baseline-gtm1-2026-07-28` |
| **Supersedes** | `stage1-baseline-gtm3-2026-07-28` (`1f68e59`) |

This document freezes the Solum workspace state that passed local `./scripts/verify.sh` and green GitHub Actions (CI, CodeQL, Secret Scan, Quality Gate) on that commit. Descriptions below are taken from crate `lib.rs` module docs, profile TOML, `deny.toml`, `.gitleaks.toml`, and `docs/` — not from aspirational product copy.

## Workspace crates

| Crate | Status | Tests | Description (from crate docs / CLI / examples) |
|-------|--------|-------|------------------------------------------------|
| `solum-core` | implementiert | lib: **17** (+1 feature-gated: **18** statt 17 mit `ferrum-storage-backend`); `tests/cli.rs`: **7**; `tests/ferrum_auth_smoke.rs`: **1**; `tests/solum_actor_auth.rs`: **2** | Product orchestration: wires jurisdiction profiles, crypto posture, audit, and clinical interchange adapters (FHIR first; openEHR staged); `Deployment` owns consent + Crypt4GH field encrypt/decrypt with matching audit events. Additive `*_as` methods accept [`SolumActor`](../crates/identity/src/lib.rs) and delegate to the unchanged `&str` APIs. `Deployment::*_as`-Methoden erzwingen jetzt Capability-Checks (`CAP_CONSENT_GRANT` / `CAP_CONSENT_REVOKE` / `CAP_CRYPTO_ENCRYPT` / `CAP_CRYPTO_DECRYPT`), fail-closed, Verweigerung schreibt `authorization.denied`-Audit-Event mit `AuditOutcome::Failure`; Legacy-`&str`-Methoden bleiben bewusst ungeprüft. Optional feature `ferrum-storage-backend`: `Deployment::with_storage` / `encrypt_field_and_store` / `read_and_decrypt_field` against Ferrum `LocalStorage` (async, kein `block_on` in der Library — Runtime bleibt beim Aufrufer); Default-Build bleibt ferrum-storage-frei. The `solum` CLI is a real tool: `consent grant` / `revoke` / `status`, `crypto encrypt` / `decrypt` (EphemeralTestKeyProvider, demo-only, Unix 0600 sidecar), `audit export` / `verify` — integration-tested via `assert_cmd`. |
| `solum-identity` | implementiert | lib: **6** | Structured actor identity adapter (`SolumActor`/`ActorSource`: FerrumPassport/Standalone/LocalDev); persisted `actor: String` format unchanged, `SolumActor` maps onto it via `to_audit_string()`. `CAP_*` Konstanten, `AuthorizationError`, `require_capability()` (fail-closed, exact-match gegen `SolumActor.scopes`). |
| `solum-auth-verify` | implementiert | lib: **6** | Standalone JWT/JWKS-Verifikation (jsonwebtoken RS256/ES256), unabhängig von privaten ferrum-core-Decode-Pfaden (keine öffentliche verify()-API in ferrum-core vorgefunden — dokumentierter Sprint-5-Rechercheergebnis); `VerifyConfig::for_ferrum_passport()` / `for_standalone_oidc()`; optionales Feature `http` für `JwksVerifier::from_url` (default aus, Offline-Pfad `from_jwks_json` zieht kein reqwest); `VerifiedClaims::into_solum_actor()` nutzt `solum-identity::ActorSource` ohne Duplikat-Enum. |
| `solum-profiles` | implementiert | lib: 12 | Jurisdiction profile loader and startup conformance checks; TOML under `config/profiles/`; mismatches refuse to start; additive `TransferPolicy` + `validate_transfer` for cross-border / secondary-use requests (restrictive-by-default). |
| `solum-crypto` | implementiert | lib: 8 +2 (feature-gated: separates Integrationstest-Target `tests/aws_kms.rs`, gemockt via aws-smithy-mocks, kein Live-AWS) | Crypt4GH envelopes for clinical field categories; customer-held key providers; same format as Ferrum genomic objects. Optional feature `aws-kms`: `AwsKmsKeyProvider` (KMS-Envelope für Crypt4GH-X25519-Seeds, asynchrones Unwrap-once beim Konstruieren, synchrones `Crypt4ghKeyProvider`-Trait unverändert — kein `block_on`); Default-Build bleibt AWS-frei, auch für Test-Targets (`required-features`-Muster). |
| `solum-audit` | implementiert | lib: 6 | Audit event recording and HELIOS-oriented evidence export hooks; in-memory `AuditLog` plus durable hash-chained `FileAuditStore`. |
| `solum-consent` | implementiert | lib: 7 | Consent and access-rights engine: grant/revoke per `(subject, purpose)` with full history for EEHRxF-style individual rights. |
| `solum-fhir` | implementiert | lib: **8** | IPS-oriented Patient Summary: FHIR R4 Bundle export inkl. bdl-9/bdl-10-Invarianten, Composition.author, Crypt4GH encrypt/decrypt über `solum-crypto` (`STAGE = "1-patient-summary"`); optionales `mii_validation_ref`-Passthrough-Feld (Composition.extension, Solum-internes ANNAHME-Provisorium für die Extension-URL). |
| `solum-openehr` | Scaffold | lib: 1 | openEHR adapter surface (stage 2 scaffold); intentionally minimal while stage 1 focuses on FHIR (`STAGE = "2-scaffold"`). |
| `solum-example-ferrum-companion` | Referenz (kein Produktcode) | binary smoke (via `verify.sh` §7 / §7b) | Mode-B-Referenz: bidirektionale Crypt4GH-Formatkompatibilität mit Ferrum + AuthClaims-Konstruktions-Smoke; optional `--features storage-backend` beweist LocalStorage Round-Trip über `Deployment` async APIs. Kein Produktcode. |

Total lib unit tests in this baseline run (default features): **71**. Plus `solum-core` integration tests: **7** CLI (`assert_cmd`) + **1** AuthClaims smoke + **2** SolumActor auth. Combined automated count referenced above: **81** (plus empty doc-test suites). With `--features ferrum-storage-backend`, `solum-core` lib is **18** (+1 LocalStorage round-trip). With `--features aws-kms`, `solum-crypto` adds **+2** mocked KMS integration tests (`tests/aws_kms.rs`). Reference deployments in `verify.sh` §7 / §7b are additional living checks (not counted in the lib unit total).

## Seit `stage1-baseline-gtm3-2026-07-28` hinzugekommen

- **GTM-1 abgeschlossen:** rollenbasierte Autorisierung für alle vier `*_as`-Methoden (`grant_consent_as` / `revoke_consent_as` / `encrypt_field_as` / `decrypt_field_as`), fail-closed (leere Scopes = immer verweigert), jede Verweigerung wird auditiert (`authorization.denied` / `AuditOutcome::Failure`).
- **Bewiesen:** Encrypt-Capability impliziert **nicht** Decrypt-Capability (separater Test) — keine implizite Rechte-Vererbung zwischen Operationen.
- **Bewiesen:** Verweigerung erzeugt keine Seiteneffekte (kein Consent-Grant, keine Ver-/Entschlüsselung bei fehlender Capability).

## Verifizierter Zustand

All `./scripts/verify.sh` sections (including §7 and §7b) passed on 2026-07-28 against commit `3d822b7ec52acae348115efd8a56a86f8f14941d` (exit 0). Section 5 emitted a long series of `cargo deny` `warning[duplicate]` trees (not failures) that are omitted below.

```
== 0. Sanity: ferrum-core pin consistency ==
ok: both pin 27a6a8e9a719fd1a171da28b20462a777f95cf65
== 1. Toolchain ==
1.91.1-aarch64-apple-darwin (overridden by '/Users/SynapticFour/devel/SynapticFour/Solum/rust-toolchain.toml')
== 2. fmt ==
== 3. clippy (deny warnings) ==
== 4. test ==
solum-audit: 6 passed
solum-auth-verify: 6 passed
solum-consent: 7 passed
solum-core lib: 17 passed
solum-core tests/cli.rs: 7 passed
solum-core tests/ferrum_auth_smoke.rs: 1 passed
solum-core tests/solum_actor_auth.rs: 2 passed
solum-crypto: 8 passed
solum-fhir: 8 passed
solum-identity: 6 passed
solum-openehr: 1 passed
solum-profiles: 12 passed
== 5. cargo-deny (licenses + sources + bans + advisories) ==
licenses ok
warning[unmatched-source]: allow-git Ferrum unmatched under local .cargo path-patch (CI uses git pin)
sources ok
(… bans duplicate-version warnings omitted …)
bans ok
advisories ok
== 6. CLI smoke test ==
ok: profile 'eu-ehds' (jurisdiction EU) matches runtime configuration
ok: non-EU storage region correctly refused
== 7. Reference deployments ==
ok: default workspace tree has no ferrum-storage
ok: standalone reference deployment (Mode A) passed
ok: AuthClaims Jwt fixture constructible (sub/issuer/scope)
ok: Crypt4GH interop (Ferrum-path ↔ Solum encrypt_field) for patient_summary
ok: ferrum-companion reference deployment (Mode B) passed
ok: both reference deployments passed
== 7b. Ferrum-storage backend (feature-gated) ==
solum-core --features ferrum-storage-backend --lib: 18 passed
ok: LocalStorage encrypt_field_and_store ↔ read_and_decrypt_field for patient_summary
ok: ferrum-companion reference deployment (Mode B) passed
ok: ferrum-storage-backend feature path passed

All baseline checks passed.
```

### Green CI runs (same commit)

| Workflow | Run ID | URL |
|----------|--------|-----|
| CI | 30390550791 | https://github.com/SynapticFour/Solum/actions/runs/30390550791 |
| CodeQL | 30390550736 | https://github.com/SynapticFour/Solum/actions/runs/30390550736 |
| Secret Scan | 30390550785 | https://github.com/SynapticFour/Solum/actions/runs/30390550785 |
| Quality Gate | 30390550762 | https://github.com/SynapticFour/Solum/actions/runs/30390550762 |

## Bewusst akzeptierte Risiken

### RUSTSEC-2023-0071 (`rsa` / Marvin Attack)

From [`deny.toml`](../deny.toml) `[advisories].ignore` and [LICENSE-COMPATIBILITY.md](../LICENSE-COMPATIBILITY.md):

- **ID:** RUSTSEC-2023-0071
- **Reason (deny.toml):** Same rsa advisory via two jsonwebtoken paths: (1) transitive jsonwebtoken ← ferrum-core (RSA-signed JWT verification); (2) direct jsonwebtoken dependency of solum-auth-verify (rust_crypto feature, RS256 path). Not via solum-crypto's own Crypt4GH field encryption. No new risk — one advisory ID, two reference sources. No upstream fix available yet. Tracked upstream in Ferrum; revisit when ferrum-core migrates away from RSA-based JWT or a patched rsa crate ships. See LICENSE-COMPATIBILITY.md / this entry for the accepted-risk record.
- **Revisit when:** `ferrum-core` migrates away from RSA-based JWT, or a patched `rsa` crate ships — then drop the ignore and re-run `cargo deny check advisories`.

### Gitleaks allowlist — Crypt4GH PEM armor headers

From [`.gitleaks.toml`](../.gitleaks.toml) (`private-key` rule allowlist, `condition = "AND"`):

- **What:** Regex `-----BEGIN CRYPT4GH (PUBLIC|PRIVATE) KEY-----` only when path is `third_party/crypt4gh/src/keys.rs`.
- **Reason:** Static PEM armor string literals in `generate_keys()` write path — not embedded key material; identical to upstream EGA-archive/crypt4gh-rust armor headers.
- **Revisit when:** Upstream crypt4gh key format / keygen writing changes, or gitleaks `private-key` rule no longer matches these literals — then re-scan and tighten or remove the allowlist.

### Kenya profile — draft, pending legal review

`kenya-dpa.toml` is present and loadable but **not production-ready**. Open items from that file’s `regulatory.notes` / `docs/profiles.md` “Kenya draft” section:

1. Retention periods (7300 days) follow Digital Health Act s.25 for the integrated system; for standalone private deployments outside that system, DPA s.39 basis is an assumption pending legal review.
2. Audit-log retention (7300 days) has no ODPC-specified figure found; aligned to clinical retention floor as a conservative assumption.
3. `required_purposes` (`research`, `public_health`, `health_insurance`) follow ODPC health guidance direction, not a codified statutory list — pending legal review.
4. Cross-border transfer is only **partially** modelled via `[transfer]` (`safeguards_based`, `statutory_exception`, `requires_serving_copy = true`); primary residency remains KE-only.
5. Digital Health Act national Health Data Bank submission obligations remain operator responsibility outside Solum’s current scope (serving-copy flag is declarative only).

### Kenya `TransferPolicy.permitted_destinations` empty

`permitted_destinations = []` in `kenya-dpa.toml`. `validate_transfer` therefore **rejects every concrete destination** even when the mechanism is listed — until ODPC case-by-case guidance fills the list. Do not treat a listed mechanism as an executable transfer permit.

### IPS / FHIR structural assumptions (`solum-fhir` Patient Summary)

From `crates/fhir/src/patient_summary.rs` (`ANNAHME` markers). These are stage-1 structural choices, **not** claimed IPS IG conformance — fachlich durch FHIR/IPS-erfahrene Person zu prüfen, bevor production-ready:

1. **Composition.type LOINC** `60591-5` (“Patient summary Document”) as IPS document type code.
2. **Section LOINCs:** Allergies `48765-2`, Medications `10160-0`, Problems `11450-4`.
3. **Empty required sections** use FHIR `emptyReason` (`nilknown`) rather than IPS-preferred “known absent” / “not known” clinical resources.
4. **Medications** emit `MedicationStatement` only (no `MedicationRequest` path in this binding).
5. **No terminology binding** to IPS value sets (e.g. SNOMED) — clinical codes are display/text only.
6. **`author_display`** maps to Composition.author as a display-only `Reference` (no `reference` URL / Organization entry).

### CLI crypto — EphemeralTestKeyProvider / plaintext key sidecar

CLI crypto subcommands use EphemeralTestKeyProvider exclusively; the `*.ephemeral-keypair.json` sidecar holds raw private key bytes in plaintext (0600 on Unix, unprotected on Windows). Not suitable for real patient data. Revisit when CLI gains real CustomerHeld / HSM-backed key provisioning.

### `SolumActor` TryFrom — Jwt tested, Passport mapping untested

`SolumActor` `TryFrom<&AuthClaims>` covers only the `Jwt` variant in tests; the `Passport` variant is handled in the mapping code but not yet tested — open for Sprint 5 (Live-Auth-Verifikation).

### `ferrum-storage-backend` — transitive AWS S3 SDK / CI coverage gap

`ferrum-storage-backend` zieht das AWS S3 SDK transitiv (auch wenn nur `LocalStorage` genutzt wird) — Feature bleibt bewusst default-aus, damit Standalone-Builds davon unberührt bleiben. CI testet aktuell nur den Default-Pfad (ohne `--features`); der Feature-Pfad ist bisher nur lokal verifiziert, nicht durch GitHub Actions abgedeckt.

### `solum-auth-verify` — unabhängige Implementierung / kein Live-Broker in CI

`solum-auth-verify` ist eine eigenständige Implementierung, kann von Ferrums tatsächlichem privaten Verifikationsverhalten abdriften, da kein öffentlicher Vergleichspunkt existiert. Kein Live-Broker-Test in CI (nur Offline-JWKS-Fixtures, analog zu Ferrums eigenem `jwks_decode.rs`-Testmuster).

### `AwsKmsKeyProvider` — plaintext seed in memory / no EncryptionContext / no live AWS in CI

`AwsKmsKeyProvider` hält den entschlüsselten Seed als normalen `Vec<u8>` ohne explizites Zeroize-on-Drop — identisch zum bestehenden Verhalten von `CustomerHeldKeyProvider`. Keine KMS-EncryptionContext-Bindung (AAD) für zusätzliche Integritäts-/Policy-Bindung. Keine Live-AWS-Tests in CI (nur aws-smithy-mocks).

### Legacy `&str`-Methoden ohne Capability-Check / keine Capability-Wildcards

Legacy `&str`-Methoden (`grant_consent`, `revoke_consent`, `encrypt_field`, `decrypt_field`) bleiben bewusst ohne Capability-Check — Autorisierung ist nur auf den `*_as`-Pfaden erzwungen. Jeder Aufrufer, der den Legacy-Pfad nutzt, umgeht GTM-1 vollständig. Das ist eine bewusste Design-Entscheidung (siehe Doc-Kommentare an den vier Methoden), aber eine reale offene Flanke, bis ein Migrationspfad zu den `*_as`-Methoden existiert. Keine Wildcard-/Hierarchie-Unterstützung in Capabilities (nur exaktes String-Match) — z.B. kein `solum:*`-Superuser-Scope.

## Explizit außerhalb dieser Baseline

Derived from [roadmap.md](roadmap.md), [profiles.md](profiles.md), [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [helios.md](helios.md), [architecture.md](architecture.md), [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md), [GTM-READINESS.md](GTM-READINESS.md), and scaffold markers in crate docs:

| Item | Source |
|------|--------|
| Remaining jurisdiction profiles (`nigeria-ndpa.toml`, `south-africa-popia.toml`) | `config/profiles/README.md`, `docs/profiles.md` — still **Planned** (`kenya-dpa.toml` is draft-Present, not listed here) |
| `solum-fhir`: Vollständige IPS IG-Konformität, Terminologie-Bindung (SNOMED/LOINC-ValueSets), MedicationRequest-Unterstützung, FHIR-Validator-Integration bleiben offen | `crates/fhir/src/lib.rs` — `STAGE = "1-patient-summary"`; `patient_summary.rs` module docs |
| `solum-openehr` bleibt bewusst zurückgestellt (siehe Konversationsverlauf 2026-07-26: openEHR-Archetype-Unsicherheit); composition / archetype / CDR / AQL binding | `crates/openehr/src/lib.rs` — stage 2 scaffold; `docs/roadmap.md` stage 2 |
| Produktions-Key-Custody in der CLI (CustomerHeld / HSM-backed provisioning) | CLI crypto is EphemeralTestKeyProvider + demo sidecar only — see “Bewusst akzeptierte Risiken” |
| Sprint 6 aus `docs/INTEGRATION-ROADMAP.md` (Turnkey-Modus) | `docs/INTEGRATION-ROADMAP.md` — Sprint 1–5 only are inside this baseline |
| JWKS-TTL-Refresh für `from_url` (aktuell einmaliger Fetch pro Verifier-Instanz) | Sprint-5 scope; offline `from_jwks_json` path is covered; URL fetch is one-shot |
| CLI-/Deployment-Wiring für `solum-auth-verify` (aktuell nur die Verify-Crate selbst, keine Integration in Deployment/CLI) | Sprint-5 constraint; verify crate is standalone |
| S3/OpenDAL-Backends (nur LocalStorage in Sprint 4 verdrahtet) | Sprint-4 scope; Ferrum `ObjectStorage` trait is broader |
| CLI-Wiring für Storage (bewusst nicht Teil von Sprint 4) | Sprint-4 constraint; CLI `main.rs` untouched |
| CI-Abdeckung für `ferrum-storage-backend`-Feature-Pfad | Local/`verify.sh` §7b only — see “Bewusst akzeptierte Risiken” |
| Patient Summary encrypt/decrypt über `Deployment` (mit `FileAuditStore`, nicht `AuditLog`) — explizit noch offen, siehe Sprint-3-Designentscheidung | Sprint-3 design note; `docs/architecture.md` FHIR/MII-Grenze |
| FHIR / IHE EEHRxF priority-category depth beyond minimal Patient Summary (labs, discharge, imaging, prescriptions) | `docs/roadmap.md` stage 2 |
| SaaS operating model | `docs/roadmap.md` stage 2; `docs/architecture.md` / PRODUCT-DEFINITION — on-premise first |
| Live HELIOS CLI/API signing integration | `docs/helios.md` — export envelope prepared; wiring is open |
| Multi-writer durable audit backend | `crates/audit/src/store.rs` — single-writer assumption for stage 1; multi-writer called stage-2 scope |
| Clinical interpretation / diagnosis / therapy support | Out of scope both stages — `docs/roadmap.md`, CONTRIBUTING MDCG boundary |
| Kenya production-ready legal closure | Draft profile inside baseline; see “Bewusst akzeptierte Risiken” — not a closed jurisdiction package |
| Wire Patient Summary encrypt/decrypt into `Deployment` / typed FHIR CLI surface | Stage-1 binding lives in `solum-fhir`; generic field encrypt/decrypt is on the CLI, typed Patient Summary path remains open |
| GTM-4 (Kunden-Doku) | `docs/GTM-READINESS.md` — GTM-1–3 inside this baseline; GTM-4 remains open |
| Migrationspfad / Deprecation für die Legacy-`&str`-Methoden | GTM-1 design: `*_as` enforced, `&str` legacy intentionally unchecked — see accepted-risk note |
| Capability-Hierarchien oder Wildcards | GTM-1 exact-match only; no `solum:*` hierarchy |
| Autorisierung in der CLI verdrahten (`main.rs` unverändert, CLI nutzt weiterhin nur die Legacy-Methoden) | GTM-1 library surface on `*_as` only; CLI wiring deferred |
| Zeroize-on-Drop für Schlüsselmaterial | Accepted-risk note above; not implemented for CustomerHeld or AwsKms |
| KMS-Provisioning-CLI (`wrap_seed` ist nur Bibliotheks-API) | GTM-3 library surface only; no CLI wrapper |
| KMS EncryptionContext/AAD-Bindung | See accepted-risk note; not wired |

Note: `docs/roadmap.md` stage-1 bullet still says “actual field-level encryption still open”; that sentence remains **stale** — Crypt4GH field encrypt/decrypt is inside this baseline (and prior ones).

## Wie diese Baseline reproduziert wird

```bash
git fetch origin tag stage1-baseline-gtm1-2026-07-28
git checkout stage1-baseline-gtm1-2026-07-28
# Prerequisites: Rust 1.91.1 (rust-toolchain.toml) and libsodium
# (e.g. brew install libsodium / apt install libsodium-dev)
./scripts/verify.sh
```

Expect all sections to pass (including §7 reference deployments and §7b feature-gated storage). This document may live on `main` at or after the tag; the tag itself points at the verified code commit listed in the header. Prior freezes: `stage1-baseline-gtm3-2026-07-28`, `stage1-baseline-sprint5-2026-07-27`, `stage1-baseline-sprint4-2026-07-27`, `stage1-baseline-sprint3-2026-07-27`, `stage1-baseline-sprint2-2026-07-27`, `stage1-baseline-sprint1-2026-07-26`, `stage1-baseline-cli-2026-07-26`, `stage1-baseline-fhir-2026-07-26`, `stage1-baseline-transfer-2026-07-26`, `stage1-baseline-2026-07-25`.
