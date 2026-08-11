# Compliance Worked Example

Reproducible **Track A** proof that Solum enforces capability-gated consent/crypto and produces a verifiable audit chain — without Ferrum, without network, without real PHI.

Full narrative (persona, claims/non-claims, expected events, known gaps):
[`docs/WORKED-EXAMPLE.md`](../../docs/WORKED-EXAMPLE.md).

## Run

From the Solum repository root:

```bash
./examples/compliance-worked-example/run.sh
```

Artifacts land in `artifacts/run-<UTC>/` (gitignored). `artifacts/latest` points at the last run.

## What this proves

| Step | Expectation |
|------|-------------|
| Profile check | `eu-ehds` accepts local runtime |
| Grant → encrypt → decrypt | Round-trip OK under CustomerHeld `--keypair` |
| Encrypt without `--capability` | Fail-closed + `authorization.denied` audit |
| Revoke | Status `revoked` |
| Decrypt after revoke | See `deny-b-result.txt` — may be **gap** (crypto does not re-check consent) |
| Audit | `audit verify` → `ok` |

## Relation to `examples/standalone`

[`../standalone/`](../standalone/) is the Mode-A smoke used by `verify.sh` §7 (temp dir, happy path only). This worked example **keeps artifacts** and exercises Deny paths for evidence packs.
