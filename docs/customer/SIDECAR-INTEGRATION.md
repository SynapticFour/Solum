# Solum — HTTP Sidecar Integration

**Audience:** Engineers embedding Solum beside an existing HMIS / EHR (PHP, Python, Java, …) without linking the Rust library or shelling the CLI.
**Authoritative product state:** [docs/BASELINE.md](../BASELINE.md). This component is **Stage 1** and **not** production-battle-tested.

This document is **not** legal advice and **not** a certification claim.

---

## 1. What the sidecar is

`solum-sidecar` is a small **HTTP process** that wraps the same `Deployment` operations the CLI uses (`grant_consent_as`, `revoke_consent_as`, `query_consent_status`, `encrypt_field_as`, `decrypt_field_as`, audit export / verify). Your application speaks **JSON over HTTP** on a local bind address.

It does **not** introduce new compliance business logic. On **pilot profiles** (`eu-ehds`, `kenya-dpa`) authorization comes from org-IAM (Bearer JWT → groups → `CAP_*`); body `capability[]` is ignored. On **`dev-local` only**, JSON `capability[]` may mint scopes (same fail-closed strings as the CLI).

---

## 2. Key custody (same posture as the CLI)

### Recommended (default): CustomerHeld via `--keys-dir`

**On-prem / multi-cloud default.** No AWS (or other cloud) account is required. Same path on bare metal, Hetzner, Azure, Alibaba, AWS VPC, or custom private cloud.

For evaluations and pilots, run the sidecar with **`--keys-dir`** pointing at a directory of operator keypair JSON files — the **same layout** as `solum crypto keygen`:

```bash
# Generate a keypair file (CLI)
cargo run -p solum-core -- crypto keygen \
  --key-ref customer/hmis-1 \
  --out /secure/solum-keys/customer_hmis-1.json
# Protect the file (0600 on Unix). Place it (or copies) under --keys-dir.

export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
export SOLUM_STORAGE_REGION=EU
cargo run -p solum-sidecar -- \
  --profile config/profiles/eu-ehds.toml \
  --audit /tmp/solum-sidecar/audit.jsonl \
  --consent-store /tmp/solum-sidecar/consent.jsonl \
  --keys-dir /secure/solum-keys \
  --org-iam-config config/org-iam/pilot-groups.toml \
  --jwks-url "$JWKS_URL" \
  --oidc-issuer "$OIDC_ISSUER" \
  --oidc-audience "$OIDC_AUDIENCE" \
  --bind 127.0.0.1:8787
```

At startup the sidecar loads **every regular file** in `--keys-dir` as JSON, registers each `key_ref` from the **file contents** (not the filename), and refuses to start on invalid JSON or an empty directory. Encrypt/decrypt only succeed for those pre-registered refs — there is **no** automatic key generation in CustomerHeld mode.

AWS KMS (optional feature `aws-kms`, **not** the default): `--wrapped-keys-dir` loads `solum crypto wrap-seed` JSON; CustomerHeld custody with `provider=aws-kms`. Seeds unwrap into process memory (ZeroizeOnDrop) — envelope, **not** HSM/TEE. Requires rebuild with `--features aws-kms`, rustc **≥ 1.94.1**, and env credentials (`AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`). Azure Key Vault / Alibaba / other KMS: **not wired** — stay on `--keys-dir`.

### Not for production: gated `--ephemeral`

> **⚠ Ephemeral test keys**
>
> `--ephemeral` uses **`EphemeralTestKeyProvider`**: keys live only in process memory, are lost on restart, and are **not** suitable for real patient data or **paid evaluations**. Requires **`SOLUM_ALLOW_EPHEMERAL=1`** (or `true`/`yes`) **and** a profile that allows `ephemeral_test` (e.g. `config/profiles/dev-local.toml`). Pilot profiles (`eu-ehds`, `kenya-dpa`) refuse EphemeralTest custody at startup. Crypto responses include a `warning` field and `X-Solum-Ephemeral-Keys`.
>
> (Same honesty posture as [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) §4; [BASELINE.md](../BASELINE.md))

Either `--keys-dir` **or** `--ephemeral` is required (clap conflict if both). Omitting both fails at startup.

**`key_ref` reuse (ephemeral only):** The first encrypt for a given `key_ref` generates a session keypair; later encrypts with the same `key_ref` reuse it (no silent rotation). CustomerHeld never auto-generates.

---

## 3. Access control (three layers)

| Layer | Mechanism | Failure |
|-------|-----------|---------|
| **Sidecar gate** | Shared secret in header `X-Solum-Sidecar-Token` (env `SOLUM_SIDECAR_TOKEN`) | **401** — request never reaches `Deployment` |
| **Capabilities** | **Pilot profiles (`eu-ehds`, `kenya-dpa`):** org-IAM Bearer JWT (`--org-iam-config`, `--jwks-url` or `--jwks-file`, `--oidc-issuer`, `--oidc-audience`). Body `capability[]` is **ignored**. **`dev-local` only:** JSON `capability[]` may mint scopes. | **401/403** — no side effect |
| **Consent + object bind** | Header/body `subject` + purpose must match an active grant **and** the FHIR / EHR / AQL object must belong to that subject | **403** `object_not_bound` / **400** AQL |

Default bind is **`127.0.0.1`**. Non-loopback binds are **refused** at startup. Terminate TLS at a reverse proxy in front of loopback. The sidecar is not a TLS terminator. Docker eval (`dev-local` only) may set `SOLUM_ALLOW_PLAINTEXT_HTTP=1` to bind `0.0.0.0` on an internal compose network.

Pilot profiles also require `SOLUM_STORAGE_REGION` (operator residency attestation). Unset → refuse to start. See [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) § operator environment.

GET `/v1/consent/status`, `/v1/audit/export`, and `/v1/audit/verify` require actor identity (`X-Solum-Actor` + `X-Solum-Capability` on `dev-local`, or Bearer JWT on pilot profiles) and the matching capability (`solum:consent:read`, `solum:audit:export`, `solum:audit:verify`).

---

## 4. Run the sidecar

**Prerequisites:** same as the CLI (Rust toolchain, libsodium). Build from source — see [RELEASING.md](../../RELEASING.md) for the SemVer binary release channel when tagged.

```bash
export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
export SOLUM_STORAGE_REGION=EU
export PROFILE=config/profiles/eu-ehds.toml
export AUDIT=/tmp/solum-sidecar/audit.jsonl
export CONSENT=/tmp/solum-sidecar/consent.jsonl
export KEYS=/secure/solum-keys
mkdir -p /tmp/solum-sidecar "$KEYS"

cargo run -p solum-sidecar -- \
  --profile "$PROFILE" \
  --audit "$AUDIT" \
  --consent-store "$CONSENT" \
  --keys-dir "$KEYS" \
  --org-iam-config config/org-iam/pilot-groups.toml \
  --jwks-url "$JWKS_URL" \
  --oidc-issuer "$OIDC_ISSUER" \
  --oidc-audience "$OIDC_AUDIENCE" \
  --bind 127.0.0.1:8787
```

For local demos only:

```bash
export SOLUM_ALLOW_EPHEMERAL=1
cargo run -p solum-sidecar -- \
  --profile config/profiles/dev-local.toml \
  --audit "$AUDIT" --consent-store "$CONSENT" \
  --ephemeral --bind 127.0.0.1:8787
```

---

## 5. curl examples

GET composition / FHIR / subject-link reads send identity in headers, **not** the query string:

- `X-Solum-Actor`
- `X-Solum-Capability` (comma-separated CAP strings)
- `X-Solum-Subject`
- `X-Solum-Purpose`

CDR/FHIR writes that touch a patient also require JSON `subject` + `purpose` and an active consent grant. Template upload remains capability-only.

### Consent grant / status / revoke

The grant/revoke bodies below use `capability[]` — that path is **`dev-local` only**. On `eu-ehds` / `kenya-dpa` send `Authorization: Bearer` instead (next subsection).

```bash
TOKEN=replace-with-a-long-random-secret
BASE=http://127.0.0.1:8787

curl -sS -X POST "$BASE/v1/consent/grant" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -d '{
    "subject": "patient/42",
    "purpose": "care_provision",
    "actor": "practitioner/7",
    "capability": ["solum:consent:grant"],
    "scope": ["patient_summary"]
  }'

curl -sS "$BASE/v1/consent/status?subject=patient%2F42&purpose=care_provision" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "X-Solum-Actor: practitioner/7" \
  -H "X-Solum-Capability: solum:consent:read"
# → {"status":"granted"|"revoked"|"unknown"}
# Pilot profiles: send Authorization: Bearer <jwt> instead of X-Solum-Capability
# (the JWT's groups must map to solum:consent:read).
```

**Ferrum (H2.1 Teeth):** When Ferrum is configured with `FERRUM_SOLUM__BASE_URL` pointing at this sidecar and a shared sidecar token, the gateway calls this status endpoint before bound DRS byte access and WES `POST /runs`. Only `granted` allows; `revoked` / `unknown` / unreachable sidecar → Ferrum **403**. On **pilot profiles** the call also needs a Bearer JWT whose groups map to `solum:consent:read` (token alone is not enough). See Showcase [ADR 0001](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/adr/0001-solum-ferrum-consent-access.md) and Ferrum [customer-runbook](https://github.com/SynapticFour/Ferrum/blob/main/docs/customer-runbook.md).

### Org IAM (H2.2) — required on pilot profiles

`--org-iam-config` plus `--jwks-url` or `--jwks-file`, `--oidc-issuer`, and `--oidc-audience` are **required** to start `eu-ehds` / `kenya-dpa`. Mutating and privileged GET routes ignore body `capability[]` and require `Authorization: Bearer <jwt>`. Groups (or another `claim_path`, e.g. Keycloak `realm_access.roles`) map to Solum CAP strings via TOML. Sidecar token remains required. CLI keeps `--capability` for offline ops.

Hospital packs (`--idp-profile entra | keycloak-hospital | smart-backend`) fill `--org-iam-config` and the default audience from `config/idp-profiles/` when those flags are unset. Consent/audit bind `standalone:<sub>` — **not** Ferrum Passports, **not** SMART App Launch. See [AUTH-HOSPITAL.md](../AUTH-HOSPITAL.md).

```bash
solum-sidecar \
  --idp-profile keycloak-hospital \
  --jwks-url http://localhost:8080/realms/hospital/protocol/openid-connect/certs \
  --oidc-issuer http://localhost:8080/realms/hospital \
  ...
```

Or the explicit mapping file:

```bash
solum-sidecar \
  --org-iam-config config/org-iam/pilot-groups.toml \
  --jwks-url http://localhost:8180/jwks.json \
  --oidc-issuer http://localhost:8180 \
  --oidc-audience solum-api \
  ...

curl -sS -X POST "$BASE/v1/consent/grant" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "Authorization: Bearer $OIDC_ACCESS_TOKEN" \
  -d '{
    "subject": "patient/42",
    "purpose": "care_provision",
    "actor": "display-only",
    "capability": [],
    "scope": ["patient_summary"]
  }'

curl -sS -X POST "$BASE/v1/consent/revoke" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "Authorization: Bearer $OIDC_ACCESS_TOKEN" \
  -d '{
    "subject": "patient/42",
    "purpose": "care_provision",
    "actor": "patient/42",
    "capability": []
  }'
```

Contract: Showcase [ADR 0002](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/adr/0002-solum-org-iam-cap.md).

### Field encrypt / decrypt (CustomerHeld `key_ref` must be pre-loaded)

```bash
PLAIN_B64=$(printf 'demo-plaintext' | base64)

curl -sS -X POST "$BASE/v1/crypto/encrypt" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -d "{
    \"category\": \"patient_summary\",
    \"subject\": \"patient/42\",
    \"purpose\": \"care_provision\",
    \"key_ref\": \"customer/hmis-1\",
    \"actor\": \"practitioner/7\",
    \"capability\": [\"solum:crypto:encrypt\"],
    \"plaintext_base64\": \"$PLAIN_B64\"
  }"
# Response JSON includes "field" + CustomerHeld "warning".
# Pass the returned "field" object back into decrypt (same key_ref + subject/purpose).
# Encrypt/decrypt require an active consent grant covering the category.
```

### Audit export / verify

```bash
curl -sS "$BASE/v1/audit/export" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "X-Solum-Actor: practitioner/7" \
  -H "X-Solum-Capability: solum:audit:export"
curl -sS "$BASE/v1/audit/verify" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "X-Solum-Actor: practitioner/7" \
  -H "X-Solum-Capability: solum:audit:verify"
# → {"status":"ok"}
```

### Track B CDR / FHIR / subject bridge (H3, opt-in)

FHIR/subject-link/dead-letter JSONL is Crypt4GH-encrypted at rest. `link_cdr: true` is refused (no example compositions as patient data). AQL must quote the consented subject. Live files rotate at `SOLUM_JSONL_MAX_BYTES`; the audit hash chain refuses appends above `SOLUM_AUDIT_MAX_BYTES`.

| Method | Path | Capability | Notes |
|--------|------|------------|-------|
| `POST` | `/v1/cdr/template` | `solum:cdr:write` | Upload pinned OPT |
| `POST` | `/v1/cdr/ehr` | `solum:cdr:write` | Create EHR |
| `POST` | `/v1/cdr/ehr/{ehr_id}/composition` | `solum:cdr:write` | Real composition JSON (`use_example=false` default). Example compositions are eval-only when `use_example=true`. |
| `GET` | `/v1/cdr/ehr/{ehr_id}/composition/{uid}` | `solum:cdr:read` | |
| `POST` | `/v1/cdr/aql` | `solum:cdr:read` | Allowlisted SELECT |
| `POST` | `/v1/fhir/{type}` | `solum:cdr:write` | H3.1 allowlist |
| `GET` | `/v1/fhir/{type}/{id}` | `solum:cdr:read` | |
| `POST` | `/v1/cdr/subject-link` | `solum:cdr:write` | ADR 0003 |
| `GET` | `/v1/cdr/subject-link/{id}` | `solum:cdr:read` | |

Partner contract: [PARTNER-EHR-API.md](PARTNER-EHR-API.md). Ops: [H3-EHRBASE-SPIKE.md](../H3-EHRBASE-SPIKE.md).

---

## 6. Capability strings (GTM‑1)

| Capability | Operation |
|------------|-----------|
| `solum:consent:grant` | Consent grant |
| `solum:consent:revoke` | Consent revoke |
| `solum:consent:read` | Consent status GET |
| `solum:crypto:encrypt` | Field encrypt |
| `solum:crypto:decrypt` | Field decrypt |
| `solum:cdr:write` | Track B CDR / FHIR / subject-link write |
| `solum:cdr:read` | Track B CDR / FHIR / AQL / subject-link read |
| `solum:audit:export` | Audit export GET |
| `solum:audit:verify` | Audit chain verify GET |

Encrypt does **not** imply decrypt. No wildcards. ([SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) §5)

---

## 7. Maturity / next steps

- Treat the sidecar as an integration preview: fail-closed behaviour and CustomerHeld / ephemeral gates are covered by automated HTTP tests; it is **not** marketed as a finished, production-hardened appliance.
- AWS KMS: optional `--features aws-kms` on CLI (`crypto wrap-seed`, `--wrapped-keypair`) and sidecar (`--wrapped-keys-dir`). Envelope + in-process unwrap — not HSM certification ([BASELINE.md](../BASELINE.md)).
- For security evaluation of Solum overall, start from [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) and the current baseline tag.

**Contact:** [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)
