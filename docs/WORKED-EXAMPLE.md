# Worked Example — Solum compliance proof (Track A)

**Date:** 2026-08-11
**Persona:** Praxis „Nordlicht“ (EU), profile [`config/profiles/eu-ehds.toml`](../config/profiles/eu-ehds.toml)
**Script:** [`examples/compliance-worked-example/run.sh`](../examples/compliance-worked-example/run.sh)
**Frozen code reference:** prefer a [BASELINE.md](BASELINE.md) tag for demos; re-run against `main` for current HEAD.

## What we prove / what we do not

| We prove | We do **not** claim |
|----------|---------------------|
| Reproducible CustomerHeld keygen → consent grant → Crypt4GH encrypt/decrypt → audit verify | EHDS / MDR / TI certification |
| Fail-closed crypto when `--capability` is omitted (`authorization.denied`) | Live HELIOS signing (export envelope only — [helios.md](helios.md)) |
| Consent-gated encrypt/decrypt after revoke (`consent.denied`) | Product `solum fhir` CLI (example binary / library only) |
| Consent revoke updates status to `revoked` | ISiK / gematik readiness (see [DE-FHIR-GAP.md](DE-FHIR-GAP.md)) |
| Hash-chained audit export is verifiable (`audit verify` → `ok`) | That legacy library `&str` crypto paths check consent (CLI/`*_as` only) |

## Run

```bash
./examples/compliance-worked-example/run.sh
```

Artifacts (gitignored): `examples/compliance-worked-example/artifacts/run-<UTC>/`
Pointer: `artifacts/latest` → last run.

`verify.sh` §8 invokes the same script.

## Scenario steps

1. `solum check --profile` (EU)
2. `crypto keygen` → CustomerHeld file
3. `consent grant` (+ `solum:consent:grant`) → status `granted`
4. `crypto encrypt` (`patient_summary`, `--subject` / `--purpose`)
5. `crypto decrypt` → byte-identical round-trip
6. **Deny A:** encrypt **without** `--capability` → non-zero exit + `authorization.denied`
7. `consent revoke` → status `revoked`
8. **Deny B:** decrypt after revoke → non-zero exit + `consent.denied`
9. `audit export` + `audit verify`

## Expected audit event types

| Event type | Notes |
|------------|-------|
| `consent.granted` | After step 3 |
| `data.encrypt` | Successful encrypt |
| `data.decrypt` | Happy-path decrypt only (post-revoke decrypt does not succeed) |
| `authorization.denied` | Deny A |
| `consent.revoked` | After step 7 |
| `consent.denied` | Deny B |

## Consent gate (Deny B)

`encrypt_field_as` / `decrypt_field_as` require:

1. GTM-1 capability (`solum:crypto:encrypt` / `decrypt`)
2. Active consent for `(subject, purpose)` covering the field category (empty grant scope = purpose-level; otherwise category must be listed)

Legacy `&str` `encrypt_field` / `decrypt_field` remain capability- and consent-unchecked (library-only; CLI uses `*_as`).

## Related proofs

Master map (every allowed claim → command): [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) · `./scripts/demo-claims-proof.sh`.

| Proof | Doc / command |
|-------|----------------|
| Mode A smoke (temp, happy path) | [`examples/standalone/`](../examples/standalone/) · `verify.sh` §7 |
| Track B EHRbase evidence | [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) · Solum-Demo `make smoke-h3` |
| FHIR IPS structural / validator | [FHIR-VALIDATION.md](FHIR-VALIDATION.md) |
| DE reference gap dossier | [DE-FHIR-GAP.md](DE-FHIR-GAP.md) |
| DE adapter spike (gated) | [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md) |
