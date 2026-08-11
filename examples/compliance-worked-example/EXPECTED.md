# Expected outcomes (compliance worked example)

This file is versioned. Per-run numbers live under `artifacts/run-*/event-types.txt`.

## Must observe

| Check | Pass criterion |
|-------|----------------|
| Profile | `solum check --profile config/profiles/eu-ehds.toml` exit 0 |
| Consent after grant | `granted` |
| Encrypt/decrypt | Byte-identical plaintext round-trip |
| Deny A (no `--capability`) | Non-zero exit; no ciphertext file; ≥1 `authorization.denied` |
| Consent after revoke | `revoked` |
| Audit verify | stdout `ok` |

## Deny B (decrypt after revoke)

| Result file | Meaning |
|-------------|---------|
| `deny-b-result.txt` = `denied` | Crypto path refuses when consent is revoked (enforced) |
| `deny-b-result.txt` starts with `gap` | **Documented gap:** GTM-1 capability checks apply; active consent is **not** re-checked on encrypt/decrypt |

As of the Proof Path introduction, Deny B is expected to report **gap** until a deliberate product change gates crypto on `consent.is_granted`.

## Typical event types (order may vary)

- `consent.granted`
- `data.encrypt` (Success)
- `data.decrypt` (Success)
- `authorization.denied` (Deny A)
- `consent.revoked`
- Possibly a further `data.decrypt` (Success) if Deny B is still a gap

## Claims

Allowed: reproducible policy enforcement + audit chain for this scenario.
Forbidden: EHDS certification, TI/ISiK readiness, live HELIOS signing.
