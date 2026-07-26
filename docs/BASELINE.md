# Stage 1 baseline (FHIR Patient Summary)

| | |
|---|---|
| **Date** | 2026-07-26 |
| **Verified commit** | `32fd0f4b225a91b1f121ed0ca8f9c7d1395aa2fd` |
| **Tag** | `stage1-baseline-fhir-2026-07-26` |
| **Supersedes** | `stage1-baseline-transfer-2026-07-26` (`a4b27c4`) |

This document freezes the Solum workspace state that passed local `./scripts/verify.sh` and green GitHub Actions (CI, CodeQL, Secret Scan, Quality Gate) on that commit. Descriptions below are taken from crate `lib.rs` module docs, profile TOML, `deny.toml`, `.gitleaks.toml`, and `docs/` — not from aspirational product copy.

## Workspace crates

| Crate | Status | Tests (lib unit) | Description (from crate docs) |
|-------|--------|------------------|-------------------------------|
| `solum-core` | implementiert | 8 | Product orchestration: wires jurisdiction profiles, crypto posture, audit, and clinical interchange adapters (FHIR first; openEHR staged); `Deployment` owns consent + Crypt4GH field encrypt/decrypt with matching audit events. |
| `solum-profiles` | implementiert | 12 | Jurisdiction profile loader and startup conformance checks; TOML under `config/profiles/`; mismatches refuse to start; additive `TransferPolicy` + `validate_transfer` for cross-border / secondary-use requests (restrictive-by-default). |
| `solum-crypto` | implementiert | 8 | Crypt4GH envelopes for clinical field categories; customer-held key providers; same format as Ferrum genomic objects. |
| `solum-audit` | implementiert | 6 | Audit event recording and HELIOS-oriented evidence export hooks; in-memory `AuditLog` plus durable hash-chained `FileAuditStore`. |
| `solum-consent` | implementiert | 7 | Consent and access-rights engine: grant/revoke per `(subject, purpose)` with full history for EEHRxF-style individual rights. |
| `solum-fhir` | implementiert | 6 | IPS-oriented Patient Summary: FHIR R4 Bundle export inkl. bdl-9/bdl-10-Invarianten, Composition.author, Crypt4GH encrypt/decrypt über `solum-crypto` (`STAGE = "1-patient-summary"`). |
| `solum-openehr` | Scaffold | 1 | openEHR adapter surface (stage 2 scaffold); intentionally minimal while stage 1 focuses on FHIR (`STAGE = "2-scaffold"`). |

Total lib unit tests in this baseline run: **48** (plus empty doc-test suites).

## Seit `stage1-baseline-transfer-2026-07-26` hinzugekommen

- **`solum-fhir`:** erste echte FHIR-Resource-Bindung (Patient Summary, IPS-orientiert), Bundle-Export, Verschlüsselung des Solum-Modells (nicht des Bundle-JSON — Bundle ist deterministisch daraus ableitbar), Cross-Profile-Test dass `patient_summary` in `eu-ehds.toml` UND `kenya-dpa.toml` als `required_field_category` geführt wird.

## Verifizierter Zustand

All six `./scripts/verify.sh` sections passed on 2026-07-26 against commit `32fd0f4b225a91b1f121ed0ca8f9c7d1395aa2fd` (exit 0). Section 5 emitted a long series of `cargo deny` `warning[duplicate]` trees (not failures) that are omitted below.

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
solum-core: 8 passed
solum-crypto: 8 passed
solum-fhir: 6 passed
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

All baseline checks passed.
```

### Green CI runs (same commit)

| Workflow | Run ID | URL |
|----------|--------|-----|
| CI | 30208129857 | https://github.com/SynapticFour/Solum/actions/runs/30208129857 |
| CodeQL | 30208129861 | https://github.com/SynapticFour/Solum/actions/runs/30208129861 |
| Secret Scan | 30208129863 | https://github.com/SynapticFour/Solum/actions/runs/30208129863 |
| Quality Gate | 30208129868 | https://github.com/SynapticFour/Solum/actions/runs/30208129868 |

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

## Explizit außerhalb dieser Baseline

Derived from [roadmap.md](roadmap.md), [profiles.md](profiles.md), [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [helios.md](helios.md), [architecture.md](architecture.md), and scaffold markers in crate docs:

| Item | Source |
|------|--------|
| Remaining jurisdiction profiles (`nigeria-ndpa.toml`, `south-africa-popia.toml`) | `config/profiles/README.md`, `docs/profiles.md` — still **Planned** (`kenya-dpa.toml` is draft-Present, not listed here) |
| `solum-fhir`: Vollständige IPS IG-Konformität, Terminologie-Bindung (SNOMED/LOINC-ValueSets), MedicationRequest-Unterstützung, FHIR-Validator-Integration bleiben offen | `crates/fhir/src/lib.rs` — `STAGE = "1-patient-summary"`; `patient_summary.rs` module docs |
| `solum-openehr`: composition / archetype / CDR / AQL binding | `crates/openehr/src/lib.rs` — stage 2 scaffold; `docs/roadmap.md` stage 2 |
| FHIR / IHE EEHRxF priority-category depth beyond minimal Patient Summary (labs, discharge, imaging, prescriptions) | `docs/roadmap.md` stage 2 |
| SaaS operating model | `docs/roadmap.md` stage 2; `docs/architecture.md` / PRODUCT-DEFINITION — on-premise first |
| Live HELIOS CLI/API signing integration | `docs/helios.md` — export envelope prepared; wiring is open |
| Multi-writer durable audit backend | `crates/audit/src/store.rs` — single-writer assumption for stage 1; multi-writer called stage-2 scope |
| Clinical interpretation / diagnosis / therapy support | Out of scope both stages — `docs/roadmap.md`, CONTRIBUTING MDCG boundary |
| Kenya production-ready legal closure | Draft profile inside baseline; see “Bewusst akzeptierte Risiken” — not a closed jurisdiction package |
| Wire Patient Summary encrypt/decrypt into `Deployment` / product CLI surface | Stage-1 binding lives in `solum-fhir`; core orchestration wiring remains open |

Note: `docs/roadmap.md` stage-1 bullet still says “actual field-level encryption still open”; that sentence remains **stale** — Crypt4GH field encrypt/decrypt is inside this baseline (and prior ones).

## Wie diese Baseline reproduziert wird

```bash
git fetch origin tag stage1-baseline-fhir-2026-07-26
git checkout stage1-baseline-fhir-2026-07-26
# Prerequisites: Rust 1.91.1 (rust-toolchain.toml) and libsodium
# (e.g. brew install libsodium / apt install libsodium-dev)
./scripts/verify.sh
```

Expect all six sections to pass. This document may live on `main` at or after the tag; the tag itself points at the verified code commit listed in the header. Prior freezes: `stage1-baseline-transfer-2026-07-26`, `stage1-baseline-2026-07-25`.
