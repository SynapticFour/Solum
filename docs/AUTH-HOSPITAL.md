# Hospital identity (standalone — no Ferrum Passport)

Solum binds **consent and audit** to the clinician or system identity your hospital IdP already issues. It does **not** mint GA4GH Passports. Ferrum remains the research plane; this pack is for Mode A next to an existing KIS.

**Not SMART App Launch.** No EHR iframe, no `launch` / `launch/patient` session. Clinician login is ordinary OIDC. Bulk/system traffic is **SMART Backend Services** (client credentials; `sub` = client_id).

## Profiles

Run the sidecar from the **repository root** so `config/idp-profiles/` resolves.

| `--idp-profile` | IdP | Groups / roles claim | Default audience |
|-----------------|-----|----------------------|------------------|
| `keycloak-hospital` | Keycloak realm (hospital) | `realm_access.roles` | `solum-api` |
| `entra` | Microsoft Entra ID | `groups` on the **access** token | `api://solum` |
| `smart-backend` | SMART Backend Services | `groups` (or map `claim_path` in org-IAM) | `solum-api` |

`--idp-profile` fills `--org-iam-config` (and the default audience) when those flags are unset. You still set `--oidc-issuer`, `--jwks-url` or `--jwks-file`, and the sidecar token. Override `--oidc-audience` / `--org-iam-config` if the site differs.

```bash
export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
export SOLUM_STORAGE_REGION=EU
cargo run -p solum-sidecar -- \
  --profile config/profiles/eu-ehds.toml \
  --audit /tmp/solum-sidecar/audit.jsonl \
  --consent-store /tmp/solum-sidecar/consent.jsonl \
  --keys-dir /secure/solum-keys \
  --idp-profile keycloak-hospital \
  --oidc-issuer https://idp.klinik.de/realms/hospital \
  --jwks-url https://idp.klinik.de/realms/hospital/protocol/openid-connect/certs \
  --bind 127.0.0.1:8787
```

Successful grants write `actor` as `standalone:<sub>` (for example `standalone:arzt-42`). That string is what consent records and the audit chain store — not a Passport visa.

## Keycloak for hospitals

Compose eval: [`docker-compose.keycloak-hospital.yml`](../docker-compose.keycloak-hospital.yml) (`docker compose -f docker-compose.keycloak-hospital.yml up -d`). Create a realm, confidential client with audience `solum-api`, and realm roles that match [`config/org-iam/keycloak-hospital.toml`](../config/org-iam/keycloak-hospital.toml) (`solum-consent-ops`, `solum-crypto-ops`, `solum-cdr-ops`, `solum-audit-ops`). Put those roles on the access token (`realm_access.roles`).

SAML from the hospital IdP terminates on Keycloak. Solum talks OIDC only.

## Entra ID

Register an app whose **access** token includes group IDs or names that match [`config/org-iam/entra.toml`](../config/org-iam/entra.toml). Do not rely on ID-token-only groups. Issuer looks like `https://login.microsoftonline.com/<tenant>/v2.0`. Scope is the API (`api://solum`), **not** `ga4gh_passport_v1`.

## SMART Backend Services

System-to-system only: client credentials against the KIS/authorization server, JWKS verify with `VerifyConfig::for_smart_backend_services` (same knobs as hospital OIDC: `iss` + `aud`). Map the client’s groups or roles to CAP strings. This is **not** SMART App Launch and **not** a Ferrum Passport broker.

## FHIR R4 with the existing KIS

Solum is the compliance layer beside the KIS you already run. Exchange FHIR R4 (IPS-oriented Patient Summary, façade allowlist). IHE (XDS/MHD) and MII Kerndatensatz are **profiles you may stamp on resources you already have** — Solum is not an IHE affinity domain, not a MII research EHR, and not a hospital HIS.

Narrow DE mapping (identifier / name / birthDate, optional KVID-10 system, **no** IG `meta.profile`): [`examples/de-adapter/README.md`](../examples/de-adapter/README.md). Still not ISiK-, gematik-, or TI-certified. No SMC-B.

## Honesty

- No Krankenhaus-EHR, no Medizinprodukt claim.
- No Passport minting in Solum.
- Pin `config/ci/ferrum-revision.txt` stays the shared crypto pin — do not bump it for this pack.
