# Stage 1 baseline (HEAD honesty — consent-gated crypto + proof path)

| | |
|---|---|
| **Date** | 2026-08-11 |
| **Verified commit** | `e17630af7aed577ed3d6a68448f2db114a3f8ceb` |
| **Tag** | *(no new freeze tag in this refresh; prior: `stage1-baseline-sidecar-custody-2026-08-01`)* |
| **Supersedes (doc honesty)** | Custody baseline `3742851` / tag `stage1-baseline-sidecar-custody-2026-08-01`; also corrects post-tag drift (H3 Track B, H2.4 KMS wiring, proof path) |

> This refresh documents **current `main` honesty** after consent-gated crypto (Deny B closed), proof-path docs, and Track B / KMS landings that outpaced the 2026-08-05 freeze text. Prefer a future annotated tag if you need a bit-for-bit evaluation pin.

This document freezes descriptions of the Solum workspace that passed local verification on the verified commit. Descriptions are taken from crate docs, profile TOML, and `docs/` — not aspirational product copy.

Push-triggered workflows: **CI** and **Secret Scan** (CodeQL weekly-only).

## Workspace crates

| Crate | Status | Tests | Description (from crate docs / CLI / examples) |
|-------|--------|-------|------------------------------------------------|
| `solum-core` | implementiert | lib: **17** (+1 feature-gated: **18** statt 17 mit `ferrum-storage-backend`); `tests/cli.rs`: **11**; `tests/ferrum_auth_smoke.rs`: **1**; `tests/solum_actor_auth.rs`: **2** | Product orchestration: wires jurisdiction profiles, crypto posture, audit, and clinical interchange adapters (FHIR first; openEHR staged); `Deployment` owns consent + Crypt4GH field encrypt/decrypt with matching audit events. Additive `*_as` methods accept [`SolumActor`](../crates/identity/src/lib.rs) and delegate to the unchanged `&str` APIs. `Deployment::*_as`-Methoden erzwingen Capability-Checks (`CAP_CONSENT_GRANT` / `CAP_CONSENT_REVOKE` / `CAP_CRYPTO_ENCRYPT` / `CAP_CRYPTO_DECRYPT`), fail-closed (`access.denied`). **Crypto `*_as` additionally requires an active consent grant** for `(subject, purpose)` covering the field category (`consent.denied` on miss/revoke). Legacy-`&str`-Methoden bleiben bewusst ungeprüft (capability **and** consent). CLI/sidecar crypto require `--subject` / `--purpose` (or JSON equivalents). Die CLI ruft ausschließlich die capability-geprüften `*_as`-Methoden auf (`--actor` + `--capability`, mehrfach wiederholbar). Fail-closed: `--capability` weglassen → leere Scopes → Verweigerung. Legacy-`&str`-APIs bleiben nur noch für Library-Integratoren erreichbar, nicht mehr über die CLI. Optional feature `ferrum-storage-backend`: `Deployment::with_storage` / `encrypt_field_and_store` / `read_and_decrypt_field` against Ferrum `LocalStorage` (async, kein `block_on` in der Library — Runtime bleibt beim Aufrufer); Default-Build bleibt ferrum-storage-frei. The `solum` CLI is a real tool: `consent grant` / `revoke` / `status`, `crypto keygen` / `encrypt` / `decrypt`, `audit export` / `verify` — integration-tested via `assert_cmd` (inkl. Deny-Test ohne `--capability`). **Phase C:** neuer Subcommand `crypto keygen` schreibt Operator-Keypair-JSON via `generate_operator_keypair()` und registriert Material für `CustomerHeldKeyProvider` (Unix **0600** via `chmod_owner_rw`, gleiches Muster wie die Ephemeral-Sidecar-Datei). `crypto encrypt` / `decrypt` verlangen standardmäßig `--keypair` (CustomerHeld); `--ephemeral` nur mit doppeltem Gate (`SOLUM_ALLOW_EPHEMERAL=1` **und** Profil mit `ephemeral_test`-Custody, z.B. `config/profiles/dev-local.toml`). EU-/Kenya-Profile lehnen `EphemeralTest`-Custody strukturell ab (Wiederverwendung von `validate_startup` aus Sprint 1). |
| `solum-sidecar` | implementiert | lib+bin: **0** unit; `tests/http.rs`: **27** | HTTP-Sidecar für Nicht-Rust-HMIS/EHR-Integratoren: wrappt Deployments capability-geprüfte `*_as`-Methoden 1:1 über REST (axum). Zwei Zugriffsschichten: Shared-Secret-Header (`X-Solum-Sidecar-Token`) mit constant-time compare, dann GTM-1-Capability-Check (H2.2 optional org-IAM: Bearer JWT groups → CAP_*). Default-Bind `127.0.0.1`. **CustomerHeld-Schlüsselverwaltung ist jetzt der Default-Pfad** (`--keys-dir`, lädt `solum crypto keygen`-JSON-Dateien, fail-closed: unlesbare/ungültige Dateien und doppelte `key_ref`-Werte brechen den Start ab, kein stilles Überspringen). `--ephemeral` bleibt hinter demselben Doppel-Gate wie die CLI (`SOLUM_ALLOW_EPHEMERAL=1` + Profil mit `ephemeral_test`-Custody). `SidecarKeys`-Enum dispatcht zwischen beiden Modi über das bestehende `Crypt4ghKeyProvider`-Trait, kein neues Custody-Modell. Ephemeral `key_exists`-Check verhindert stillschweigendes Schlüssel-Überschreiben bei `key_ref`-Wiederverwendung innerhalb einer Laufzeit (CustomerHeld generiert nie automatisch). |
| `solum-identity` | implementiert | lib: **9** | Structured actor identity adapter (`SolumActor`/`ActorSource`: FerrumPassport/Standalone/LocalDev); persisted `actor: String` format unchanged, `SolumActor` maps onto it via `to_audit_string()`. `CAP_*` Konstanten, `AuthorizationError`, `require_capability()` (fail-closed, exact-match gegen `SolumActor.scopes`). |
| `solum-auth-verify` | implementiert | lib: **8** | Standalone JWT/JWKS-Verifikation (jsonwebtoken RS256/ES256), unabhängig von privaten ferrum-core-Decode-Pfaden (keine öffentliche verify()-API in ferrum-core vorgefunden — dokumentierter Sprint-5-Rechercheergebnis); `VerifyConfig::for_ferrum_passport()` / `for_standalone_oidc()`; optionales Feature `http` für `JwksVerifier::from_url` (default aus, Offline-Pfad `from_jwks_json` zieht kein reqwest); `VerifiedClaims::into_solum_actor()` nutzt `solum-identity::ActorSource` ohne Duplikat-Enum. |
| `solum-profiles` | implementiert | lib: 12 | Jurisdiction profile loader and startup conformance checks; TOML under `config/profiles/` (inkl. `dev-local.toml` für gated ephemeral demos); mismatches refuse to start; additive `TransferPolicy` + `validate_transfer` for cross-border / secondary-use requests (restrictive-by-default). |
| `solum-crypto` | implementiert | lib: **9** +2 (feature-gated: separates Integrationstest-Target `tests/aws_kms.rs`, gemockt via aws-smithy-mocks, kein Live-AWS) | Crypt4GH envelopes for clinical field categories; customer-held key providers; `generate_operator_keypair` for operator-supplied CustomerHeld files; same format as Ferrum genomic objects. **Default custody = on-prem CustomerHeld files** (cloud-agnostic). Optional feature `aws-kms`: `AwsKmsKeyProvider` + CLI/sidecar wiring (H2.4) — AWS CMK envelope only; not Azure/Alibaba/Hetzner providers. Default-Build bleibt AWS-frei (`required-features`-Muster). |
| `solum-audit` | implementiert | lib: 6 | Audit event recording and HELIOS-oriented evidence export hooks; in-memory `AuditLog` plus durable hash-chained `FileAuditStore`. Live HELIOS signing is **deferred** (see [helios.md](helios.md)); export envelopes remain. |
| `solum-consent` | implementiert | lib: 7 | Consent and access-rights engine: grant/revoke per `(subject, purpose)` with full history for EEHRxF-style individual rights. |
| `solum-fhir` | implementiert | lib: **8** | IPS-oriented Patient Summary: FHIR R4 Bundle export inkl. bdl-9/bdl-10-Invarianten, Composition.author, Crypt4GH encrypt/decrypt über `solum-crypto` (`STAGE = "1-patient-summary"`); optionales `mii_validation_ref`-Passthrough-Feld (Composition.extension, Solum-internes ANNAHME-Provisorium für die Extension-URL). |
| `solum-openehr` | implementiert (Track B opt-in) | lib + ignored live smoke | EHRbase REST client (`STAGE = "3.1-fhir-aql"`): EHR/OPT/composition/AQL allowlist. Sidecar `--ehrbase-url` + `/v1/cdr/*`. **Not** a semantic FHIR↔openEHR mapper — presence/smoke compositions + façade. Hub-class only. |
| `solum-example-ferrum-companion` | Referenz (kein Produktcode) | binary smoke (via `verify.sh` §7 / §7b) | Mode-B-Referenz: bidirektionale Crypt4GH-Formatkompatibilität mit Ferrum + AuthClaims-Konstruktions-Smoke; optional `--features storage-backend` beweist LocalStorage Round-Trip über `Deployment` async APIs. Kein Produktcode. |

Total lib unit tests (default features): recount on verify — `solum-core` lib includes consent-gate coverage; sidecar **0** lib units / **27** HTTP tests. Plus CLI **11** + AuthClaims **1** + SolumActor **2**. Feature-gated: `ferrum-storage-backend` (+1), `aws-kms` (+2 mocked). `verify.sh` §7/§7b/§8 are additional living checks.

## Seit custody freeze / proof path (through 2026-08-11)

- **Consent-gated crypto (Deny B closed):** `encrypt_field_as` / `decrypt_field_as` require active grant covering category; CLI `--subject`/`--purpose`; sidecar encrypt/decrypt JSON fields; audit `consent.denied`. Worked example enforces post-revoke decrypt refusal ([WORKED-EXAMPLE.md](WORKED-EXAMPLE.md), issue #1).
- **Proof path:** compliance worked example + `verify.sh` §8; H3 evidence packaging; IPS structural PASS + HL7 Validator + `hl7.fhir.uv.ips#2.0.0` campaign **Success** (0 errors / 0 warnings after UUID/LOINC/ait-1/narrative harden — [FHIR-VALIDATION.md](FHIR-VALIDATION.md)); remaining IPS `ANNAHME`s still open; [DE-FHIR-GAP.md](DE-FHIR-GAP.md) + pilot-gated [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md). No product `solum fhir` CLI — example binary / library only.
- **Track B / H3 (post-custody-tag):** EHRbase façade, AQL, FHIR JSONL store, subject-link, dual-write webhook — see [H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md). Prior freeze text calling openEHR a `2-scaffold` is **stale**.
- **H2.4 AWS KMS:** optional `--features aws-kms` CLI `wrap-seed` / `--wrapped-keypair` and sidecar `--wrapped-keys-dir`. New wraps bind KMS **EncryptionContext** (`solum:purpose` + `solum:key_ref`); legacy empty-context files still unwrap. Honesty limits: in-memory seed after unwrap, mocked CI (no live AWS).
- **Legacy `&str` Deployment crypto/consent:** `#[deprecated]` — use `*_as` (capability + consent for crypto).
- **Typed Patient Summary on Deployment:** `encrypt_patient_summary_as` / `decrypt_patient_summary_as` write durable audit; CLI `solum fhir export-ips` for Bundle export.
- **Sidecar↔CLI CustomerHeld parity** (custody tag `3742851`) remains in force.


## Verifizierter Zustand

Local checks on 2026-08-11 against `ae1592b`: full `./scripts/verify.sh` (incl. §8) **passed**; HL7 IPS Validator campaign **Success** after Bundle harden. Claims map: [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) · operator one-shot `./scripts/demo-claims-proof.sh`.

Prior freeze Kurzfassung (2026-08-05 / `3742851`) omitted here — see git history for that snapshot.

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

### Kenya profile — provisional (counsel still required)

`kenya-dpa.toml` is loadable as **PROVISIONAL-PRODUCTION-CANDIDATE** after a **non-counsel** Vorprüfung (an internal non-counsel Vorprüfung). **Not** production SoR / **not** ODPC-certified. Real counsel via external counsel review remains mandatory before PRODUCTION.

Honesty after Vorprüfung:

1. Retention `7300` days = conservative Digital Health Act–aligned default — **not** a universal private-sector statutory mandate.
2. Audit-log `7300` = security evidence retention; no ODPC-specified figure claimed.
3. `required_purposes` = primary-care floor; `research` / secondary use in `optional_purposes` only (separate lawful basis).
4. Cross-border: mechanisms listed as pathways; `permitted_destinations = []` fail-closed until TIA + approval.
5. National Health Data Bank submission remains operator responsibility (Solum non-goal).
6. Edge offline residency / revoke / key policies are documented in `regulatory.notes`; field enforcement is K2/K3.

### Kenya `TransferPolicy.permitted_destinations` empty

`permitted_destinations = []` in `kenya-dpa.toml`. `validate_transfer` therefore **rejects every concrete destination** even when the mechanism is listed — until ODPC case-by-case guidance fills the list. Do not treat a listed mechanism as an executable transfer permit.

### IPS / FHIR structural assumptions (`solum-fhir` Patient Summary)

From `crates/fhir/src/patient_summary.rs` (`ANNAHME` markers). These are stage-1 structural choices, **not** claimed IPS IG conformance — fachlich durch FHIR/IPS-erfahrene Person zu prüfen, bevor production-ready:

1. **Composition.type LOINC** `60591-5` with official display **Patient Summary** as IPS document type code.
2. **Section LOINCs:** Allergies `48765-2`, Medications `10160-0`, Problems `11450-4`.
3. **Empty required sections** use FHIR `emptyReason` (`nilknown`) rather than IPS-preferred “known absent” / “not known” clinical resources.
4. **Medications** emit `MedicationStatement` only (no `MedicationRequest` path in this binding).
5. **No terminology binding** to IPS value sets (e.g. SNOMED) — clinical codes are display/text only.
6. **`author_display`** maps to Composition.author referencing an Organization Bundle entry (fullUrl + display).

### CLI crypto — CustomerHeld `--keypair` vs gated ephemeral

Evaluation / pilot CLI path: `crypto keygen` + `--keypair` (CustomerHeld file, 0600 on Unix). Ephemeral (`--ephemeral`) requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile allowing `ephemeral_test` (`dev-local.toml`); pilot profiles refuse `EphemeralTest` custody. Ephemeral sidecars remain plaintext JSON (0600 on Unix). Not an HSM. Private seeds in `CustomerHeldKeyProvider` / `AwsKmsKeyProvider` use **best-effort `ZeroizeOnDrop`** (H2) — not a TEE.

### AWS-KMS — optional CLI / sidecar feature (H2.4)

Feature `aws-kms` on `solum-core` / `solum-sidecar` (and `solum-crypto`):

- CLI: `solum crypto wrap-seed`, encrypt/decrypt `--wrapped-keypair`
- Sidecar: `--wrapped-keys-dir` / `SOLUM_SIDECAR_WRAPPED_KEYS_DIR`
- Custody remains `CustomerHeld` with `provider=aws-kms`
- Credentials via env (`AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` [+ optional session token]) — not full IRSA/`aws-config` chain (MSRV posture)
- Build/test this feature with **rustc ≥ 1.94.1** (AWS SDK); default Solum MSRV **1.91.1** builds stay AWS-free
- Honesty: envelope unwrap into process memory (`ZeroizeOnDrop`); **not** HSM/TEE/FIPS certification; mocked tests only in CI (`cargo test -p solum-crypto --features aws-kms --test aws_kms`)

### HELIOS live signing — deferred

Live HELIOS CLI/API signing is **deferred / not productized** for Stage‑1 evaluations — see [docs/helios.md](helios.md). Hash-chained audit store and HELIOS-oriented export envelopes remain; do not claim live signing or turnkey HELIOS bridge.

### `SolumActor` TryFrom — Jwt + Passport tested

`SolumActor` `TryFrom<&AuthClaims>` covers `Jwt` and `Passport` variants in `crates/core/tests/solum_actor_auth.rs` (visas are not mapped into scopes — only `claims.scope`).

### `ferrum-storage-backend` — transitive AWS S3 SDK / CI coverage gap

`ferrum-storage-backend` zieht das AWS S3 SDK transitiv (auch wenn nur `LocalStorage` genutzt wird) — Feature bleibt bewusst default-aus, damit Standalone-Builds davon unberührt bleiben. CI testet aktuell nur den Default-Pfad (ohne `--features`); der Feature-Pfad ist bisher nur lokal verifiziert, nicht durch GitHub Actions abgedeckt.

### `solum-auth-verify` — unabhängige Implementierung / kein Live-Broker in CI

`solum-auth-verify` ist eine eigenständige Implementierung, kann von Ferrums tatsächlichem privaten Verifikationsverhalten abdriften, da kein öffentlicher Vergleichspunkt existiert. Kein Live-Broker-Test in CI (nur Offline-JWKS-Fixtures, analog zu Ferrums eigenem `jwks_decode.rs`-Testmuster).

### `AwsKmsKeyProvider` — plaintext seed in memory / mocked CI

`AwsKmsKeyProvider` hält den entschlüsselten Seed in Prozessspeicher mit **best-effort `ZeroizeOnDrop`** (wie `CustomerHeldKeyProvider`). New wraps send KMS **EncryptionContext** (`solum:purpose=crypt4gh-seed`, `solum:key_ref=…`); legacy on-disk JSON without context still decrypts without context. Keine Live-AWS-Tests in CI (nur aws-smithy-mocks).

### Legacy LIBRARY actor paths skip authorization / keine Capability-Wildcards

Legacy library `&str`-Methoden (`grant_consent`, `revoke_consent`, `encrypt_field`, `decrypt_field`) bleiben bewusst ohne Capability-Check — Autorisierung ist auf den `*_as`-Pfaden erzwungen. Die **CLI** nutzt diesen Legacy-Pfad **nicht mehr** (sie baut einen structured Actor aus `--actor` + `--capability` und ruft nur `*_as` auf; weglassen von `--capability` → fail-closed). Jeder **Library**-Aufrufer, der den Legacy-`&str`-Pfad nutzt, umgeht GTM-1 weiterhin vollständig. Das ist eine bewusste Design-Entscheidung (siehe Doc-Kommentare an den vier Methoden), aber eine reale offene Flanke für Library-Integratoren, bis ein Migrationspfad zu den `*_as`-Methoden existiert. Keine Wildcard-/Hierarchie-Unterstützung in Capabilities (nur exaktes String-Match) — z.B. kein `solum:*`-Superuser-Scope.

## Explizit außerhalb dieser Baseline

Derived from [roadmap.md](roadmap.md), [profiles.md](profiles.md), [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [helios.md](helios.md), [architecture.md](architecture.md), [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md), and scaffold markers in crate docs:

| Item | Source |
|------|--------|
| Remaining jurisdiction profiles (`nigeria-ndpa.toml`, `south-africa-popia.toml`) | `config/profiles/README.md`, `docs/profiles.md` — still **Planned** (`kenya-dpa.toml` is draft-Present, not listed here) |
| `solum-fhir`: Vollständige IPS IG-Konformität, Terminologie-Bindung (SNOMED/LOINC-ValueSets), MedicationRequest-Unterstützung, FHIR-Validator-Integration bleiben offen | `crates/fhir/src/lib.rs` — `STAGE = "1-patient-summary"`; `patient_summary.rs` module docs |
| Nur ein FHIR-Ressourcentyp (Patient Summary) — labs, discharge, imaging, prescriptions und weitere EEHRxF-Priority-Kategorien bleiben offen | `docs/roadmap.md` stage 2; `crates/fhir` Patient-Summary-only binding |
| `solum-openehr` bleibt bewusst zurückgestellt (siehe Konversationsverlauf 2026-07-26: openEHR-Archetype-Unsicherheit); composition / archetype / CDR / AQL binding | `crates/openehr/src/lib.rs` — stage 2 scaffold; `docs/roadmap.md` stage 2 |
| HSM-backed key custody / production HSM CLI | CustomerHeld file `--keypair` / sidecar `--keys-dir` is wired; no HSM |
| Azure Key Vault / Alibaba KMS / Hetzner / custom cloud KMS providers | Use CustomerHeld files; only AWS CMK envelope is wired (`aws-kms`) |
| GitHub Release binaries before first verified `v*` tag | Workflow prepared (`.github/workflows/release.yml`); until a SemVer tag succeeds, install from source |
| Ferrum-Storage / Auth-Verify nur als Referenzbeispiel, nicht im Produktpfad verdrahtet | `solum-example-ferrum-companion` + optional `ferrum-storage-backend` / standalone `solum-auth-verify` — reference/library surfaces, not a turnkey product path |
| Sprint 6 aus `docs/INTEGRATION-ROADMAP.md` (Turnkey-Modus) | `docs/INTEGRATION-ROADMAP.md` — Sprint 1–5 only are inside this baseline |
| JWKS-TTL-Refresh für `from_url` (aktuell einmaliger Fetch pro Verifier-Instanz) | Sprint-5 scope; offline `from_jwks_json` path is covered; URL fetch is one-shot |
| CLI-/Deployment-Wiring für `solum-auth-verify` (aktuell nur die Verify-Crate selbst, keine Integration in Deployment/CLI) | **H2.2:** sidecar org-IAM wires `solum-auth-verify` + group→CAP mapping; CLI still uses `--capability` |
| S3/OpenDAL-Backends (nur LocalStorage in Sprint 4 verdrahtet) | Sprint-4 scope; Ferrum `ObjectStorage` trait is broader |
| CLI-Wiring für Storage (bewusst nicht Teil von Sprint 4) | Sprint-4 constraint; storage remains library/`Deployment` feature path |
| CI-Abdeckung für `ferrum-storage-backend`-Feature-Pfad | Local/`verify.sh` §7b only — see “Bewusst akzeptierte Risiken” |
| Patient Summary encrypt/decrypt über `Deployment` (mit `FileAuditStore`) | **Done** — `encrypt_patient_summary_as` / `decrypt_patient_summary_as` |
| FHIR / IHE EEHRxF priority-category depth beyond minimal Patient Summary (labs, discharge, imaging, prescriptions) | `docs/roadmap.md` stage 2 |
| SaaS operating model | `docs/roadmap.md` stage 2; `docs/architecture.md` / PRODUCT-DEFINITION — on-premise first |
| Live HELIOS CLI/API signing integration | `docs/helios.md` — **deferred / not productized**; export envelope only |
| Multi-writer durable audit backend | `crates/audit/src/store.rs` — single-writer assumption for stage 1; multi-writer called stage-2 scope |
| Clinical interpretation / diagnosis / therapy support | Out of scope both stages — `docs/roadmap.md`, CONTRIBUTING MDCG boundary |
| Kenya production-ready legal closure | Provisional profile inside baseline; counsel still required — see “Bewusst akzeptierte Risiken” |
| Nigeria / South Africa production profiles | DRAFT scaffolds under `config/profiles/planned/` only — not auto-loaded |
| Wire Patient Summary encrypt/decrypt into `Deployment` / typed FHIR CLI surface | **Done** — `encrypt_patient_summary_as` / `decrypt_patient_summary_as`; `solum fhir export-ips` |
| Migrationspfad / Deprecation für die Legacy-`&str`-Methoden (Library) | **`#[deprecated]` landed**; removal still open — CLI already on `*_as` |
| Capability-Hierarchien oder Wildcards | GTM-1 exact-match only; no `solum:*` hierarchy |
| Zeroize-on-Drop für Schlüsselmaterial | Best-effort `ZeroizeOnDrop` on held seeds (H2); not a TEE |
| KMS EncryptionContext/AAD-Bindung | **Done** for new wraps (`seed_encryption_context`); legacy empty-context files supported |

Note: `docs/roadmap.md` stage-1 bullet still says “actual field-level encryption still open”; that sentence remains **stale** — Crypt4GH field encrypt/decrypt is inside this baseline (and prior ones). CLI authorization, binary release workflow, CustomerHeld CLI path, and sidecar CustomerHeld/`--keys-dir` parity are **inside** this baseline; first production SemVer tag and AWS-KMS CLI/sidecar wiring remain outside.

## Wie diese Baseline reproduziert wird

```bash
git fetch origin tag stage1-baseline-sidecar-custody-2026-08-01
git checkout stage1-baseline-sidecar-custody-2026-08-01
# Prerequisites: Rust 1.91.1 (rust-toolchain.toml) and libsodium
# (e.g. brew install libsodium / apt install libsodium-dev)
./scripts/verify.sh
```

Expect all sections to pass (including §7 reference deployments with CustomerHeld `--keypair` on the standalone example, and §7b feature-gated storage). This document may live on `main` at or after the tag; the tag itself points at the verified code commit listed in the header. Prior freezes: `stage1-baseline-sidecar-2026-07-30`, `stage1-baseline-website-2026-07-30`, `stage1-baseline-cli-authz-2026-07-29`, `stage1-baseline-gtm4-2026-07-28`, `stage1-baseline-gtm1-2026-07-28`, `stage1-baseline-gtm3-2026-07-28`, `stage1-baseline-sprint5-2026-07-27`, `stage1-baseline-sprint4-2026-07-27`, `stage1-baseline-sprint3-2026-07-27`, `stage1-baseline-sprint2-2026-07-27`, `stage1-baseline-sprint1-2026-07-26`, `stage1-baseline-cli-2026-07-26`, `stage1-baseline-fhir-2026-07-26`, `stage1-baseline-transfer-2026-07-26`, `stage1-baseline-2026-07-25`.
