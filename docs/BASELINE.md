# Stage 1 baseline (Sprint 2 SolumActor identity adapter)

| | |
|---|---|
| **Date** | 2026-07-27 |
| **Verified commit** | `c72e71de24f1a616bd2146f6c6423c12831faf88` |
| **Tag** | `stage1-baseline-sprint2-2026-07-27` |
| **Supersedes** | `stage1-baseline-sprint1-2026-07-26` (`e3f15c4`) |

This document freezes the Solum workspace state that passed local `./scripts/verify.sh` and green GitHub Actions (CI, CodeQL, Secret Scan, Quality Gate) on that commit. Descriptions below are taken from crate `lib.rs` module docs, profile TOML, `deny.toml`, `.gitleaks.toml`, and `docs/` — not from aspirational product copy.

## Workspace crates

| Crate | Status | Tests | Description (from crate docs / CLI / examples) |
|-------|--------|-------|------------------------------------------------|
| `solum-core` | implementiert | lib: **9**; `tests/cli.rs`: **7**; `tests/ferrum_auth_smoke.rs`: **1**; `tests/solum_actor_auth.rs`: **2** | Product orchestration: wires jurisdiction profiles, crypto posture, audit, and clinical interchange adapters (FHIR first; openEHR staged); `Deployment` owns consent + Crypt4GH field encrypt/decrypt with matching audit events. Additive `*_as` methods accept [`SolumActor`](../crates/identity/src/lib.rs) and delegate to the unchanged `&str` APIs. The `solum` CLI is a real tool: `consent grant` / `revoke` / `status`, `crypto encrypt` / `decrypt` (EphemeralTestKeyProvider, demo-only, Unix 0600 sidecar), `audit export` / `verify` — integration-tested via `assert_cmd`. |
| `solum-identity` | implementiert | lib: **4** | Structured actor identity adapter (`SolumActor`/`ActorSource`: FerrumPassport/Standalone/LocalDev); persisted `actor: String` format unchanged, `SolumActor` maps onto it via `to_audit_string()`. |
| `solum-profiles` | implementiert | lib: 12 | Jurisdiction profile loader and startup conformance checks; TOML under `config/profiles/`; mismatches refuse to start; additive `TransferPolicy` + `validate_transfer` for cross-border / secondary-use requests (restrictive-by-default). |
| `solum-crypto` | implementiert | lib: 8 | Crypt4GH envelopes for clinical field categories; customer-held key providers; same format as Ferrum genomic objects. |
| `solum-audit` | implementiert | lib: 6 | Audit event recording and HELIOS-oriented evidence export hooks; in-memory `AuditLog` plus durable hash-chained `FileAuditStore`. |
| `solum-consent` | implementiert | lib: 7 | Consent and access-rights engine: grant/revoke per `(subject, purpose)` with full history for EEHRxF-style individual rights. |
| `solum-fhir` | implementiert | lib: 6 | IPS-oriented Patient Summary: FHIR R4 Bundle export inkl. bdl-9/bdl-10-Invarianten, Composition.author, Crypt4GH encrypt/decrypt über `solum-crypto` (`STAGE = "1-patient-summary"`). |
| `solum-openehr` | Scaffold | lib: 1 | openEHR adapter surface (stage 2 scaffold); intentionally minimal while stage 1 focuses on FHIR (`STAGE = "2-scaffold"`). |
| `solum-example-ferrum-companion` | Referenz (kein Produktcode) | binary smoke (via `verify.sh` §7) | Sprint-1 Mode-B-Referenz, beweist bidirektionale Crypt4GH-Formatkompatibilität mit Ferrum + AuthClaims-Konstruktions-Smoke, kein Produktcode. |

Total lib unit tests in this baseline run: **53**. Plus `solum-core` integration tests: **7** CLI (`assert_cmd`) + **1** AuthClaims smoke + **2** SolumActor auth. Combined automated count referenced above: **63** (plus empty doc-test suites). Reference deployments in `verify.sh` §7 are additional living checks (not counted in the lib unit total).

## Seit `stage1-baseline-sprint1-2026-07-26` hinzugekommen

- **Sprint 2 abgeschlossen:** `solum-identity` Crate, `SolumActor`/`ActorSource`, `TryFrom<&AuthClaims>` hinter Feature `ferrum-companion`, `Deployment::grant_consent_as` / `revoke_consent_as` / `encrypt_field_as` / `decrypt_field_as` (additiv, bestehende `&str`-APIs unverändert).
- **Bewiesen:** FerrumPassport- und Standalone-Actor erzeugen identisch strukturierte Audit-Events (gleicher `event_type` / `data_category` / `outcome` / `details`, nur `actor`-String unterscheidet sich) — Test `grant_consent_as_ferrum_and_standalone_same_audit_shape`.
- **Bewiesen:** `SolumActor::from(String)::to_audit_string()` ist bit-identisch zum Original-String — Rückwärtskompatibilität mit allen bisherigen Baselines/Audit-Exports.

## Verifizierter Zustand

All seven `./scripts/verify.sh` sections passed on 2026-07-27 against commit `c72e71de24f1a616bd2146f6c6423c12831faf88` (exit 0). Section 5 emitted a long series of `cargo deny` `warning[duplicate]` trees (not failures) that are omitted below.

```
== 0. Sanity: ferrum-core pin consistency ==
ok: both pin 27a6a8e9a719fd1a171da28b20462a777f95cf65
== 1. Toolchain ==
1.91.1-aarch64-apple-darwin (overridden by '/Users/SynapticFour/devel/SynapticFour/Solum/rust-toolchain.toml')
== 2. fmt ==
== 3. clippy (deny warnings) ==
== 4. test ==
solum-audit: 6 passed
solum-consent: 7 passed
solum-core lib: 9 passed
solum-core tests/cli.rs: 7 passed
solum-core tests/ferrum_auth_smoke.rs: 1 passed
solum-core tests/solum_actor_auth.rs: 2 passed
solum-crypto: 8 passed
solum-fhir: 6 passed
solum-identity: 4 passed
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
ok: standalone reference deployment (Mode A) passed
ok: AuthClaims Jwt fixture constructible (sub/issuer/scope)
ok: Crypt4GH interop (Ferrum-path ↔ Solum encrypt_field) for patient_summary
ok: ferrum-companion reference deployment (Mode B) passed
ok: both reference deployments passed

All baseline checks passed.
```

### Green CI runs (same commit)

| Workflow | Run ID | URL |
|----------|--------|-----|
| CI | 30242011672 | https://github.com/SynapticFour/Solum/actions/runs/30242011672 |
| CodeQL | 30242011717 | https://github.com/SynapticFour/Solum/actions/runs/30242011717 |
| Secret Scan | 30242011744 | https://github.com/SynapticFour/Solum/actions/runs/30242011744 |
| Quality Gate | 30242011706 | https://github.com/SynapticFour/Solum/actions/runs/30242011706 |

## Bewusst akzeptierte Risiken

### RUSTSEC-2023-0071 (`rsa` / Marvin Attack)

From [`deny.toml`](../deny.toml) `[advisories].ignore` and [LICENSE-COMPATIBILITY.md](../LICENSE-COMPATIBILITY.md):

- **ID:** RUSTSEC-2023-0071
- **Reason (deny.toml):** Transitive via `jsonwebtoken` ← `ferrum-core` (RSA-signed JWT verification), not via `solum-crypto`'s own Crypt4GH field encryption. No upstream fix available yet. Tracked upstream in Ferrum.
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

## Explizit außerhalb dieser Baseline

Derived from [roadmap.md](roadmap.md), [profiles.md](profiles.md), [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [helios.md](helios.md), [architecture.md](architecture.md), [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md), and scaffold markers in crate docs:

| Item | Source |
|------|--------|
| Remaining jurisdiction profiles (`nigeria-ndpa.toml`, `south-africa-popia.toml`) | `config/profiles/README.md`, `docs/profiles.md` — still **Planned** (`kenya-dpa.toml` is draft-Present, not listed here) |
| `solum-fhir`: Vollständige IPS IG-Konformität, Terminologie-Bindung (SNOMED/LOINC-ValueSets), MedicationRequest-Unterstützung, FHIR-Validator-Integration bleiben offen | `crates/fhir/src/lib.rs` — `STAGE = "1-patient-summary"`; `patient_summary.rs` module docs |
| `solum-openehr` bleibt bewusst zurückgestellt (siehe Konversationsverlauf 2026-07-26: openEHR-Archetype-Unsicherheit); composition / archetype / CDR / AQL binding | `crates/openehr/src/lib.rs` — stage 2 scaffold; `docs/roadmap.md` stage 2 |
| Produktions-Key-Custody in der CLI (CustomerHeld / HSM-backed provisioning) | CLI crypto is EphemeralTestKeyProvider + demo sidecar only — see “Bewusst akzeptierte Risiken” |
| Sprint 3–6 aus `docs/INTEGRATION-ROADMAP.md` (FHIR/MII-Grenze, Storage-Wiederverwendung, Live-Auth-Verifikation, Turnkey-Modus) | `docs/INTEGRATION-ROADMAP.md` — Sprint 1–2 only are inside this baseline |
| FHIR / IHE EEHRxF priority-category depth beyond minimal Patient Summary (labs, discharge, imaging, prescriptions) | `docs/roadmap.md` stage 2 |
| SaaS operating model | `docs/roadmap.md` stage 2; `docs/architecture.md` / PRODUCT-DEFINITION — on-premise first |
| Live HELIOS CLI/API signing integration | `docs/helios.md` — export envelope prepared; wiring is open |
| Multi-writer durable audit backend | `crates/audit/src/store.rs` — single-writer assumption for stage 1; multi-writer called stage-2 scope |
| Clinical interpretation / diagnosis / therapy support | Out of scope both stages — `docs/roadmap.md`, CONTRIBUTING MDCG boundary |
| Kenya production-ready legal closure | Draft profile inside baseline; see “Bewusst akzeptierte Risiken” — not a closed jurisdiction package |
| Wire Patient Summary encrypt/decrypt into `Deployment` / typed FHIR CLI surface | Stage-1 binding lives in `solum-fhir`; generic field encrypt/decrypt is on the CLI, typed Patient Summary path remains open |

Note: `docs/roadmap.md` stage-1 bullet still says “actual field-level encryption still open”; that sentence remains **stale** — Crypt4GH field encrypt/decrypt is inside this baseline (and prior ones).

## Wie diese Baseline reproduziert wird

```bash
git fetch origin tag stage1-baseline-sprint2-2026-07-27
git checkout stage1-baseline-sprint2-2026-07-27
# Prerequisites: Rust 1.91.1 (rust-toolchain.toml) and libsodium
# (e.g. brew install libsodium / apt install libsodium-dev)
./scripts/verify.sh
```

Expect all seven sections to pass (including §7 reference deployments). This document may live on `main` at or after the tag; the tag itself points at the verified code commit listed in the header. Prior freezes: `stage1-baseline-sprint1-2026-07-26`, `stage1-baseline-cli-2026-07-26`, `stage1-baseline-fhir-2026-07-26`, `stage1-baseline-transfer-2026-07-26`, `stage1-baseline-2026-07-25`.
