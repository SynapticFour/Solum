# Solum — HTTP Sidecar Integration

**Audience:** Engineers embedding Solum beside an existing HMIS / EHR (PHP, Python, Java, …) without linking the Rust library or shelling the CLI.
**Authoritative product state:** [docs/BASELINE.md](../BASELINE.md). This component is **Stage 1** and **not** production-battle-tested.

This document is **not** legal advice and **not** a certification claim.

---

## 1. What the sidecar is

`solum-sidecar` is a small **HTTP process** that wraps the same `Deployment` operations the CLI uses (`grant_consent_as`, `revoke_consent_as`, `query_consent_status`, `encrypt_field_as`, `decrypt_field_as`, audit export / verify). Your application speaks **JSON over HTTP** on a local bind address.

It does **not** introduce new compliance business logic. Fail-closed GTM‑1 capability checks behave like the CLI (`actor` + `capability[]` → structured actor; omit capabilities → deny).

---

## 2. Key custody (same posture as the CLI)

### Recommended: CustomerHeld via `--keys-dir`

For evaluations and pilots, run the sidecar with **`--keys-dir`** pointing at a directory of operator keypair JSON files — the **same layout** as `solum crypto keygen`:

```bash
# Generate a keypair file (CLI)
cargo run -p solum-core -- crypto keygen \
  --key-ref customer/hmis-1 \
  --out /secure/solum-keys/customer_hmis-1.json
# Protect the file (0600 on Unix). Place it (or copies) under --keys-dir.

export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
cargo run -p solum-sidecar -- \
  --profile config/profiles/eu-ehds.toml \
  --audit /tmp/solum-sidecar/audit.jsonl \
  --consent-store /tmp/solum-sidecar/consent.jsonl \
  --keys-dir /secure/solum-keys \
  --bind 127.0.0.1:8787
```

At startup the sidecar loads **every regular file** in `--keys-dir` as JSON, registers each `key_ref` from the **file contents** (not the filename), and refuses to start on invalid JSON or an empty directory. Encrypt/decrypt only succeed for those pre-registered refs — there is **no** automatic key generation in CustomerHeld mode.

AWS KMS is **not** wired into the sidecar (library-only today; follow-on work).

### Not for production: gated `--ephemeral`

> **⚠ Ephemeral test keys**
>
> `--ephemeral` uses **`EphemeralTestKeyProvider`**: keys live only in process memory, are lost on restart, and are **not** suitable for real patient data or **paid evaluations**. Requires **`SOLUM_ALLOW_EPHEMERAL=1`** (or `true`/`yes`) **and** a profile that allows `ephemeral_test` (e.g. `config/profiles/dev-local.toml`). Pilot profiles (`eu-ehds`, `kenya-dpa`) refuse EphemeralTest custody at startup. Crypto responses include a `warning` field and `X-Solum-Ephemeral-Keys`.
>
> (Same honesty posture as [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) §4; [BASELINE.md](../BASELINE.md))

Either `--keys-dir` **or** `--ephemeral` is required (clap conflict if both). Omitting both fails at startup.

**`key_ref` reuse (ephemeral only):** The first encrypt for a given `key_ref` generates a session keypair; later encrypts with the same `key_ref` reuse it (no silent rotation). CustomerHeld never auto-generates.

---

## 3. Access control (two layers)

| Layer | Mechanism | Failure |
|-------|-----------|---------|
| **Sidecar gate** | Shared secret in header `X-Solum-Sidecar-Token` (env `SOLUM_SIDECAR_TOKEN`) | **401** — request never reaches `Deployment` |
| **GTM‑1 capabilities** | JSON `capability` array (exact strings, e.g. `solum:consent:grant`) | **403** — no consent/crypto side effect |

Default bind is **`127.0.0.1`** (not `0.0.0.0`). Override only via `SOLUM_SIDECAR_BIND` / `--bind` if you intentionally expose another interface — and then treat network exposure as your responsibility.

---

## 4. Run the sidecar

**Prerequisites:** same as the CLI (Rust toolchain, libsodium). Build from source — see [RELEASING.md](../../RELEASING.md) for the SemVer binary release channel when tagged.

```bash
export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
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

### Consent grant / status / revoke

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
  -H "X-Solum-Sidecar-Token: $TOKEN"
# → {"status":"granted"|"revoked"|"unknown"}

**Ferrum (H2.1 Teeth):** When Ferrum is configured with `FERRUM_SOLUM__BASE_URL` pointing at this sidecar and a shared sidecar token, the gateway calls this status endpoint before bound DRS byte access and WES `POST /runs`. Only `granted` allows; `revoked` / `unknown` / unreachable sidecar → Ferrum **403**. Status remains token-gated (no `CAP_*`). See Showcase [ADR 0001](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/adr/0001-solum-ferrum-consent-access.md) and Ferrum [customer-runbook](https://github.com/SynapticFour/Ferrum/blob/main/docs/customer-runbook.md).

curl -sS -X POST "$BASE/v1/consent/revoke" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -d '{
    "subject": "patient/42",
    "purpose": "care_provision",
    "actor": "patient/42",
    "capability": ["solum:consent:revoke"]
  }'
```

### Field encrypt / decrypt (CustomerHeld `key_ref` must be pre-loaded)

```bash
PLAIN_B64=$(printf 'demo-plaintext' | base64)

curl -sS -X POST "$BASE/v1/crypto/encrypt" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -d "{
    \"category\": \"patient_summary\",
    \"key_ref\": \"customer/hmis-1\",
    \"actor\": \"practitioner/7\",
    \"capability\": [\"solum:crypto:encrypt\"],
    \"plaintext_base64\": \"$PLAIN_B64\"
  }"
# Response JSON includes "field" + CustomerHeld "warning".
# Pass the returned "field" object back into decrypt (same key_ref).
```

### Audit export / verify

```bash
curl -sS "$BASE/v1/audit/export" -H "X-Solum-Sidecar-Token: $TOKEN"
curl -sS "$BASE/v1/audit/verify" -H "X-Solum-Sidecar-Token: $TOKEN"
# → {"status":"ok"}
```

---

## 6. Capability strings (GTM‑1)

| Capability | Operation |
|------------|-----------|
| `solum:consent:grant` | Consent grant |
| `solum:consent:revoke` | Consent revoke |
| `solum:crypto:encrypt` | Field encrypt |
| `solum:crypto:decrypt` | Field decrypt |

Encrypt does **not** imply decrypt. No wildcards. ([SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) §5)

---

## 7. Maturity / next steps

- Treat the sidecar as an integration preview: fail-closed behaviour and CustomerHeld / ephemeral gates are covered by automated HTTP tests; it is **not** marketed as a finished, production-hardened appliance.
- AWS KMS for the sidecar (and CLI) remains **follow-on** work ([BASELINE.md](../BASELINE.md)).
- For security evaluation of Solum overall, start from [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) and the current baseline tag.

**Contact:** [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)
