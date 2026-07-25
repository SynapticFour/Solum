# Stage 1 baseline

| | |
|---|---|
| **Date** | 2026-07-25 |
| **Verified commit** | `f0a4d22a5b24b84d1c55b073d04797ceedc9fa4f` |
| **Tag** | `stage1-baseline-2026-07-25` |

This document freezes the Solum workspace state that passed local `./scripts/verify.sh` and green GitHub Actions (CI, CodeQL, Secret Scan, Quality Gate) on that commit. Descriptions below are taken from crate `lib.rs` module docs, `deny.toml`, `.gitleaks.toml`, and `docs/` — not from aspirational product copy.

## Workspace crates

| Crate | Status | Tests (lib unit) | Description (from crate docs) |
|-------|--------|------------------|-------------------------------|
| `solum-core` | implementiert | 8 | Product orchestration: wires jurisdiction profiles, crypto posture, audit, and clinical interchange adapters (FHIR first; openEHR staged); `Deployment` owns consent + Crypt4GH field encrypt/decrypt with matching audit events. |
| `solum-profiles` | implementiert | 5 | Jurisdiction profile loader and startup conformance checks; TOML under `config/profiles/`; mismatches refuse to start. |
| `solum-crypto` | implementiert | 8 | Crypt4GH envelopes for clinical field categories; customer-held key providers; same format as Ferrum genomic objects. |
| `solum-audit` | implementiert | 6 | Audit event recording and HELIOS-oriented evidence export hooks; in-memory `AuditLog` plus durable hash-chained `FileAuditStore`. |
| `solum-consent` | implementiert | 7 | Consent and access-rights engine: grant/revoke per `(subject, purpose)` with full history for EEHRxF-style individual rights. |
| `solum-fhir` | Scaffold | 1 | FHIR R4/R5 adapter surface (stage 1); placeholder handle for a future FHIR client / validator binding (`STAGE = "1-scaffold"`). |
| `solum-openehr` | Scaffold | 1 | openEHR adapter surface (stage 2 scaffold); intentionally minimal while stage 1 focuses on FHIR (`STAGE = "2-scaffold"`). |

Total lib unit tests in this baseline run: **36** (plus empty doc-test suites).

## Verifizierter Zustand

All six `./scripts/verify.sh` sections passed on 2026-07-25 against commit `f0a4d22a5b24b84d1c55b073d04797ceedc9fa4f` (exit 0). Full log was 2344 lines; section 5 emitted a long series of `cargo deny` `warning[duplicate]` trees (not failures) that are omitted below.

```
== 0. Sanity: ferrum-core pin consistency ==
ok: both pin 27a6a8e9a719fd1a171da28b20462a777f95cf65
== 1. Toolchain ==
1.91.1-aarch64-apple-darwin (overridden by '/Users/SynapticFour/devel/SynapticFour/Solum/rust-toolchain.toml')
== 2. fmt ==
== 3. clippy (deny warnings) ==
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.04s
== 4. test ==
solum-audit: 6 passed
solum-consent: 7 passed
solum-core: 8 passed
solum-crypto: 8 passed
solum-fhir: 1 passed
solum-openehr: 1 passed
solum-profiles: 5 passed
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
| CI | 30169294764 | https://github.com/SynapticFour/Solum/actions/runs/30169294764 |
| CodeQL | 30169294815 | https://github.com/SynapticFour/Solum/actions/runs/30169294815 |
| Secret Scan | 30169294759 | https://github.com/SynapticFour/Solum/actions/runs/30169294759 |
| Quality Gate | 30169294814 | https://github.com/SynapticFour/Solum/actions/runs/30169294814 |

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

## Explizit außerhalb dieser Baseline

Derived from [roadmap.md](roadmap.md), [profiles.md](profiles.md), [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [helios.md](helios.md), [architecture.md](architecture.md), and scaffold markers in crate docs (grep: `scaffold` / `planned` / `Placeholder` / stage-2 notes):

| Item | Source |
|------|--------|
| Additional jurisdiction profiles (`kenya-dpa.toml`, `nigeria-ndpa.toml`, `south-africa-popia.toml`) | `config/profiles/README.md`, `docs/profiles.md`, `docs/PRODUCT-DEFINITION.md` — marked **Planned**; only `eu-ehds.toml` present |
| `solum-fhir`: real HL7 FHIR resource / client / validator binding | `crates/fhir/src/lib.rs` — `Placeholder crate`, `STAGE = "1-scaffold"` |
| `solum-openehr`: composition / archetype / CDR / AQL binding | `crates/openehr/src/lib.rs` — stage 2 scaffold; `docs/roadmap.md` stage 2 |
| FHIR / IHE EEHRxF priority-category depth (patient summary, labs, discharge, imaging, prescriptions) | `docs/roadmap.md` stage 2 |
| SaaS operating model | `docs/roadmap.md` stage 2; `docs/architecture.md` / PRODUCT-DEFINITION — on-premise first |
| Live HELIOS CLI/API signing integration | `docs/helios.md` — export envelope prepared; wiring is open |
| Multi-writer durable audit backend | `crates/audit/src/store.rs` — single-writer assumption for stage 1; multi-writer called stage-2 scope |
| Clinical interpretation / diagnosis / therapy support | Out of scope both stages — `docs/roadmap.md`, CONTRIBUTING MDCG boundary |

Note: `docs/roadmap.md` stage-1 bullet still says “actual field-level encryption still open”; that sentence is **stale relative to this baseline** — Crypt4GH field encrypt/decrypt is implemented in `solum-crypto` and wired through `solum-core::Deployment` (see crate tests). Field encryption is **inside** this baseline, not outside it.

## Wie diese Baseline reproduziert wird

```bash
git fetch origin tag stage1-baseline-2026-07-25
git checkout stage1-baseline-2026-07-25
# Prerequisites: Rust 1.91.1 (rust-toolchain.toml) and libsodium
# (e.g. brew install libsodium / apt install libsodium-dev)
./scripts/verify.sh
```

Expect all six sections to pass. This document may live on `main` at or after the tag; the tag itself points at the verified code commit listed in the header.
