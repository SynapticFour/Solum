# Demo operator

This is the operator note for a hospital-IdP demo of the sidecar. It is not a clinical pilot and not a production deployment. Demo bytes are synthetic or already public. Do not pass `--ephemeral` for that demo: ephemeral keys disappear when the process exits.

## Keycloak hospital IdP

Pack file: `config/idp-profiles/keycloak-hospital.toml`.

| Setting | Value |
|---------|--------|
| Profile name | `keycloak-hospital` |
| Issuer check | `issuer_contains = "/realms/"`. The operator sets the full issuer URL (`--oidc-issuer` / `SOLUM_ORG_IAM_ISSUER`). This file does not name a live host. |
| Audience | `solum-api` |
| Groups claim | `realm_access.roles` |
| Org-IAM map | `config/org-iam/keycloak-hospital.toml` |
| JWKS | The operator sets `--jwks-url` or `--jwks-file`. A typical Keycloak URL is `{issuer}/protocol/openid-connect/certs`. This demo was not logged in against a live realm. |

`--idp-profile keycloak-hospital` fills the org-IAM path and the default audience when those flags are unset.

## Roles to capabilities

`config/org-iam/keycloak-hospital.toml`:

| Keycloak role | Capabilities |
|---------------|----------------|
| `solum-consent-ops` | `solum:consent:grant`, `solum:consent:revoke`, `solum:consent:read` |
| `solum-crypto-ops` | `solum:crypto:encrypt`, `solum:crypto:decrypt` |
| `solum-cdr-ops` | `solum:cdr:write`, `solum:cdr:read` |
| `solum-audit-ops` | `solum:audit:export`, `solum:audit:verify` |

## Region and keys

Pilot profiles require `SOLUM_STORAGE_REGION`. For `eu-ehds` that value is `EU`. Keys for the public demo come from `--keys-dir` (customer-held files from `solum crypto keygen`). Those files have to persist across restarts. `--ephemeral` is refused when `SOLUM_ALLOW_INTERNAL_BIND` is set, and it is the wrong custody for a demo that is meant to be inspected later.

## Bind

Default bind stays `127.0.0.1`. `SOLUM_ALLOW_PLAINTEXT_HTTP=1` remains `dev-local` only.

`SOLUM_ALLOW_INTERNAL_BIND=1` or `--allow-internal-bind` is the pilot opt-in for a non-loopback bind (ADR 0004). The process exits unless the sidecar token, profile `eu-ehds` or `kenya-dpa`, and `--keys-dir` are all set. Startup warns that HTTP is plaintext and TLS stops at the proxy.

## Health

`GET /health` and `GET /ready` do not use the sidecar token. The body is status flags only: no paths, key ids, or chain error text. `/ready` is 503 when the audit file is not writable.

## Auditor

`GET /v1/audit/export` and `GET /v1/audit/verify` require the sidecar token plus `solum:audit:export` or `solum:audit:verify`. The Keycloak role `solum-audit-ops` maps to both. Export is the full HELIOS JSON, including subject and purpose. That is the auditor view. There is no unauthenticated audit route. A token without `solum-audit-ops` cannot export or verify.
