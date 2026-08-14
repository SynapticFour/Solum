# Expected outcomes (compliance worked example)

This file is versioned. Per-run numbers live under `artifacts/run-*/event-types.txt`.

## Must observe

| Check | Pass criterion |
|-------|----------------|
| Profile | `solum check --profile config/profiles/eu-ehds.toml` exit 0 |
| Consent after grant | `granted` |
| Encrypt/decrypt | Byte-identical plaintext round-trip (`--subject` / `--purpose` required) |
| Deny A (no `--capability`) | Non-zero exit; no ciphertext file; ≥1 `access.denied` |
| Consent after revoke | `revoked` |
| Deny B (decrypt after revoke) | Non-zero exit; `deny-b-result.txt` = `denied`; ≥1 `consent.denied` |
| Audit verify | stdout `ok` |

## Typical event types

- `consent.granted`
- `data.encrypt` (Success)
- `data.decrypt` (Success) — happy path only
- `access.denied` (Deny A)
- `consent.revoked`
- `consent.denied` (Deny B)

## Claims

Allowed: reproducible policy enforcement + audit chain for this scenario.
Forbidden: EHDS certification, TI/ISiK readiness, live HELIOS signing.
