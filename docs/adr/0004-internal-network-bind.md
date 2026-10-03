# ADR 0004 — Internal-network plaintext bind

- **Status:** Accepted
- **Date:** 2026-10-03
- **Product:** Solum sidecar

## Context

The sidecar speaks plaintext HTTP. The default bind is loopback, and TLS stops at a reverse proxy. A pilot Compose network needs the process to listen on a container address that is not loopback. `SOLUM_ALLOW_PLAINTEXT_HTTP=1` is the dev-local eval switch. Using it on `eu-ehds` or `kenya-dpa` is refused after the profile loads.

## Decision

`SOLUM_ALLOW_INTERNAL_BIND=1` and `--allow-internal-bind` are a separate opt-in. The default stays off.

The process refuses to start when the flag is set unless all of these are true:

- `SOLUM_SIDECAR_TOKEN` is non-empty
- the loaded profile is `eu-ehds` or `kenya-dpa`
- keys come from `--keys-dir`

`--ephemeral`, `--wrapped-keys-dir`, and `dev-local` are refused. `SOLUM_ALLOW_PLAINTEXT_HTTP` stays dev-local only and does not satisfy this flag.

When the flag is honoured on a non-loopback bind, startup logs that HTTP is plaintext and TLS must stop at the proxy.

`GET /health` and `GET /ready` sit outside the sidecar-token middleware. Bodies are status flags. They do not include paths, key ids, or chain error text. `/ready` is 503 when the audit file is not writable, the chain does not verify, or the key provider lock is poisoned.

## Consequences

A pilot container can bind `0.0.0.0` only with the token, the pilot profile, and customer-held keys. A missing piece exits. Loopback remains the default. The auditor routes `/v1/audit/export` and `/v1/audit/verify` stay behind the sidecar token and `solum:audit:export` / `solum:audit:verify`.
