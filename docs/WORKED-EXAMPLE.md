# Worked Example — Solum compliance proof (Track A)

**Date:** 2026-08-11
**Persona:** Praxis „Nordlicht“ (EU), profile [`config/profiles/eu-ehds.toml`](../config/profiles/eu-ehds.toml)
**Script:** [`examples/compliance-worked-example/run.sh`](../examples/compliance-worked-example/run.sh)
**Frozen code reference:** prefer a [BASELINE.md](BASELINE.md) tag for demos; re-run against `main` for current HEAD.

## What we prove / what we do not

| We prove | We do **not** claim |
|----------|---------------------|
| Reproducible CustomerHeld keygen → consent grant → Crypt4GH encrypt/decrypt → audit verify | EHDS / MDR / TI certification |
| Fail-closed crypto when `--capability` is omitted (`authorization.denied` in the audit chain) | Live HELIOS signing (export envelope only) |
| Consent revoke updates status to `revoked` | That encrypt/decrypt re-check active consent (see Known gaps) |
| Hash-chained audit export is verifiable (`audit verify` → `ok`) | ISiK / gematik readiness (see [DE-FHIR-GAP.md](DE-FHIR-GAP.md)) |

## Run

```bash
./examples/compliance-worked-example/run.sh
```

Artifacts (gitignored): `examples/compliance-worked-example/artifacts/run-<UTC>/`
Pointer: `artifacts/latest` → last run (`MANIFEST.txt`, `audit.jsonl`, `helios-export.json`, `event-types.txt`, `deny-b-result.txt`).

`verify.sh` §8 invokes the same script.

## Scenario steps

1. `solum check --profile` (EU)
2. `crypto keygen` → CustomerHeld file
3. `consent grant` (+ `solum:consent:grant`) → status `granted`
4. `crypto encrypt` (`patient_summary`)
5. `crypto decrypt` → byte-identical round-trip
6. **Deny A:** encrypt **without** `--capability` → non-zero exit + `authorization.denied`
7. `consent revoke` → status `revoked`
8. **Deny B:** decrypt after revoke → see Known gaps
9. `audit export` + `audit verify`

## Expected audit event types

Observed on a green local run (2026-08-11):

| Event type | Count | Notes |
|------------|-------|-------|
| `consent.granted` | 1 | After step 3 |
| `data.encrypt` | 1 | Successful encrypt |
| `data.decrypt` | 2 | Happy-path decrypt + post-revoke decrypt (while Deny B is a gap) |
| `authorization.denied` | 1 | Deny A |
| `consent.revoked` | 1 | After step 7 |

Exact counts may grow if Deny B becomes enforced (post-revoke decrypt would then be Failure or absent).

## Known gaps

### Crypto does not require active consent (Deny B)

`Deployment::encrypt_field` / `decrypt_field` enforce **GTM-1 capabilities** on `*_as` paths but do **not** call `consent.is_granted`. After revoke, decrypt with a valid capability still succeeds. The worked example records this in `deny-b-result.txt` as `gap` rather than failing the script.

**Why documented, not silently “fixed” in this proof path:** gating crypto on consent is a product design change (purpose binding, category scope, care vs secondary use). Track it as a follow-up issue; do not claim revoke blocks decryption until that lands.

### Related

- Legacy library `&str` crypto/consent APIs remain capability-unchecked (CLI uses `*_as` only).
- Live HELIOS signing deferred — see [helios.md](helios.md).

## Related proofs

| Proof | Doc / command |
|-------|----------------|
| Mode A smoke (temp, happy path) | [`examples/standalone/`](../examples/standalone/) · `verify.sh` §7 |
| Track B EHRbase evidence | [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) · Solum-Demo `make smoke-h3` |
| FHIR IPS structural / validator | [FHIR-VALIDATION.md](FHIR-VALIDATION.md) |
| DE reference gap dossier | [DE-FHIR-GAP.md](DE-FHIR-GAP.md) |
| DE adapter spike (gated) | [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md) |
