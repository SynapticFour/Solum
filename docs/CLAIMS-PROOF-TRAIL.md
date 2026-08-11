# Claims → proof trail

**Date:** 2026-08-11
**Purpose:** Single map from every Stage‑1 claim we allow to a **runnable demo or automated proof**, plus the claim we explicitly forbid next to it.
**Authoritative product state:** [BASELINE.md](BASELINE.md).

One-shot operator pass (Track A + structural FHIR + Kenya checks; optional HL7 JAR / H3 Docker):

```bash
./scripts/demo-claims-proof.sh
```

| Gate | Command |
|------|---------|
| Full CI-parity baseline | `./scripts/verify.sh` |
| Pin after green | update **Verified commit** in [BASELINE.md](BASELINE.md) |

---

## Claim matrix

| # | Allowed claim (short) | Demo / proof | Artifact / signal | Forbidden next to it |
|---|----------------------|--------------|-------------------|----------------------|
| A1 | CustomerHeld keygen → consent → Crypt4GH encrypt/decrypt → hash-chained audit verify | [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) · `./examples/compliance-worked-example/run.sh` · `verify.sh` §8 | `examples/compliance-worked-example/artifacts/run-*/` (`helios-export.json`, audit chain `ok`) | EHDS / MDR / TI certification |
| A2 | Crypto without `--capability` fails closed (`authorization.denied`) | Same worked example Deny A | Audit event `authorization.denied` | Wildcard / hierarchy scopes |
| A3 | Decrypt after revoke fails closed (`consent.denied`) | Same worked example Deny B | Audit event `consent.denied` | Legacy `&str` library path checks consent |
| A4 | Mode A standalone happy-path smoke | `./examples/standalone/run.sh` · `verify.sh` §7 | Temp workdir; audit verify `ok` | Production deployment proof |
| A5 | Ferrum Crypt4GH interop (Mode B) | `verify.sh` §7 / §7b · `examples/ferrum-companion` | Script exit 0 | Ferrum product identity / genomic SoR |
| A6 | HELIOS-**oriented** audit export envelope exists; chain verifies | Worked example step 9 / standalone `audit export` + `audit verify` | `helios-export.json` shape `solum-audit-helios-v1` | Live HELIOS signing / turnkey bridge ([helios.md](helios.md)) |
| A7 | EU profile residency / custody check | `cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml` · wrong region → non-zero ([profiles.md](profiles.md)) | CLI `ok:` / refuse message | Legal compliance certificate |
| A8 | Kenya profile loads; KE residency + CustomerHeld only; transfer destinations fail-closed | CLI checks in [profiles.md](profiles.md); unit `kenya_validate_transfer_fail_closed_empty_destinations`; sidecar HTTP Kenya refuse tests | Test / CLI non-zero on EU region or ephemeral | ODPC certification / PRODUCTION SoR / filled transfer permits |
| A9 | IPS-oriented Bundle structural PASS | [FHIR-VALIDATION.md](FHIR-VALIDATION.md) · `solum fhir export-ips` · `./scripts/validate-fhir-ips.sh` | `examples/fhir-ips-export/out/structural-check.txt` | ISiK / TI readiness |
| A10 | Same Bundle + HL7 Validator + `hl7.fhir.uv.ips#2.0.0` → 0 errors / 0 warnings (pinned campaign) | JAR in `.cache/`; `FHIR_VALIDATOR_JAR=… ./scripts/validate-fhir-ips.sh` | `out/validator-log.txt` Success | Full IPS certification beyond package pin; clinical correctness |
| A14 | Typed Patient Summary encrypt/decrypt with durable audit | Unit `encrypt_patient_summary_as_writes_audit_and_round_trips` · library `Deployment::*_patient_summary_as` | Audit `data.encrypt` / `data.decrypt` | Clinical correctness of summary contents |
| A15 | Migration Prefer/Cut-over **tooling** dry rehearsal | `./scripts/migration-rehearsal-dry-run.sh` | Inventory + dead-letter artifacts | Live partner Prefer/Cut-over without site |
| A11 | DE FHIR / ISiK gap dossier exists (competence + honesty) | [DE-FHIR-GAP.md](DE-FHIR-GAP.md) (document proof, not runtime) | Dossier tables + campaign log | “TI-konform” / gematik-zertifiziert |
| A12 | Track B: Solum façade + EHRbase smoke retains evidence when stack is up | [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) · sibling Solum-Demo `make up-h3 && make smoke-h3` | Demo `artifacts/smoke-h3/` (+ `MANIFEST.txt`) | MDR clearance; Synaptic Four EHR UI; production Keycloak |
| A13 | Nigeria / SA profiles are **draft scaffolds only** | Files under `config/profiles/planned/` + [planned/README.md](../config/profiles/planned/README.md); **not** loaded by `load_profiles_dir("config/profiles")` | Directory presence; no auto-load | Production NDPA/POPIA package |

---

## How to demo in 15 minutes (Track A)

```bash
# 1) Consent + Deny A/B + audit (≈2–3 min cold)
./examples/compliance-worked-example/run.sh

# 2) FHIR structural (always) + optional Java validator
./scripts/validate-fhir-ips.sh
# optional:
#   export FHIR_VALIDATOR_JAR="$PWD/.cache/validator_cli.jar"
#   ./scripts/validate-fhir-ips.sh

# 3) Kenya fail-closed residency (expect non-zero)
! SOLUM_STORAGE_REGION=EU cargo run -q -p solum-core -- \
    check --profile config/profiles/kenya-dpa.toml

# 4) Kenya empty transfer destinations (unit)
cargo test -p solum-profiles kenya_validate_transfer_fail_closed_empty_destinations -- --nocapture
```

Track B (Docker): see [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md).

**Solum-Demo mirror** (interactive + HTTP smokes aligned to this matrix): sibling checkout
`../Solum-Demo` — `make up-sibling`, `make smoke-stage1`, `make smoke-consent` (Deny B),
`make smoke-fhir-ips`, `make smoke-profile`, `make smoke-migration`, `make smoke-h3`.
Coverage map: [Solum-Demo docs/COVERAGE.md](https://github.com/SynapticFour/Solum-Demo/blob/main/docs/COVERAGE.md).

---

## Doc index (do not invent parallel claims)

| Doc | Role |
|-----|------|
| [BASELINE.md](BASELINE.md) | Frozen honesty + accepted risks |
| [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) | A1–A3 narrative |
| [FHIR-VALIDATION.md](FHIR-VALIDATION.md) | A9–A10 |
| [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) | A12 |
| [DE-FHIR-GAP.md](DE-FHIR-GAP.md) | A11 |
| [helios.md](helios.md) | A6 boundary |
| [profiles.md](profiles.md) | A7–A8 / A13 |
| [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) | Customer-readable restatement |
| [PRIORITIES.md](PRIORITIES.md) | What to build next (not claims) |
