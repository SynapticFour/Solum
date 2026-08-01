# Solum — HTTP Sidecar Integration

**Audience:** Engineers embedding Solum beside an existing HMIS / EHR (PHP, Python, Java, …) without linking the Rust library or shelling the CLI.
**Authoritative product state:** [docs/BASELINE.md](../BASELINE.md). This component is **Stage 1** and **not** production-battle-tested.

This document is **not** legal advice and **not** a certification claim.

---

## 1. What the sidecar is

`solum-sidecar` is a small **HTTP process** that wraps the same `Deployment` operations the CLI uses (`grant_consent_as`, `revoke_consent_as`, `query_consent_status`, `encrypt_field_as`, `decrypt_field_as`, audit export / verify). Your application speaks **JSON over HTTP** on a local bind address.

It does **not** introduce new compliance business logic. Fail-closed GTM‑1 capability checks behave like the CLI (`actor` + `capability[]` → structured actor; omit capabilities → deny).

---

## 2. Not for production data

> **⚠ Ephemeral test keys (sidecar crypto endpoints)**
>
> Crypto encrypt/decrypt use **`EphemeralTestKeyProvider` only** in this Stage‑1 sidecar: keys are **not** suitable for real patient data, live only in the sidecar **process memory** for that run, and are **lost on restart**. Production key custody (customer-held / HSM-backed / AWS KMS) is **not** wired into the sidecar. Every crypto response also includes a `warning` field and an `X-Solum-Ephemeral-Keys` header so HTTP clients that never see process logs still see the restriction.
>
> (Same honesty posture as [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) §4 for the CLI; [BASELINE.md](../BASELINE.md))

**Do not** process real patient data with the sidecar crypto endpoints in this baseline.

---

## 3. Access control (two layers)

| Layer | Mechanism | Failure |
|-------|-----------|---------|
| **Sidecar gate** | Shared secret in header `X-Solum-Sidecar-Token` (env `SOLUM_SIDECAR_TOKEN`) | **401** — request never reaches `Deployment` |
| **GTM‑1 capabilities** | JSON `capability` array (exact strings, e.g. `solum:consent:grant`) | **403** — no consent/crypto side effect |

Default bind is **`127.0.0.1`** (not `0.0.0.0`). Override only via `SOLUM_SIDECAR_BIND` / `--bind` if you intentionally expose another interface — and then treat network exposure as your responsibility.

---

## 4. Run the sidecar

**Prerequisites:** same as the CLI (Rust toolchain, libsodium). Build from source — no packaged binary channel is documented in this repository.

```bash
export SOLUM_SIDECAR_TOKEN='replace-with-a-long-random-secret'
export PROFILE=config/profiles/eu-ehds.toml
export AUDIT=/tmp/solum-sidecar/audit.jsonl
export CONSENT=/tmp/solum-sidecar/consent.jsonl
mkdir -p /tmp/solum-sidecar

cargo run -p solum-sidecar -- \
  --profile "$PROFILE" \
  --audit "$AUDIT" \
  --consent-store "$CONSENT" \
  --bind 127.0.0.1:8787
```

Expect a startup warning about ephemeral keys on stderr / logs.

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

### Field encrypt / decrypt (demo keys only — see §2)

```bash
PLAIN_B64=$(printf 'demo-plaintext' | base64)

curl -sS -X POST "$BASE/v1/crypto/encrypt" \
  -H "Content-Type: application/json" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -d "{
    \"category\": \"patient_summary\",
    \"key_ref\": \"ephemeral/demo-1\",
    \"actor\": \"practitioner/7\",
    \"capability\": [\"solum:crypto:encrypt\"],
    \"plaintext_base64\": \"$PLAIN_B64\"
  }"
# Response JSON includes "field" + "warning"; header X-Solum-Ephemeral-Keys is set.

# Pass the returned "field" object back into decrypt (same sidecar process / key_ref).
```

**`key_ref` reuse within one sidecar process:** The first encrypt for a given `key_ref` generates an ephemeral keypair and keeps it in memory. Later encrypts with the **same** `key_ref` reuse that keypair (no silent rotation). Ciphertexts from earlier encrypts with that `key_ref` stay decryptable until the process exits. Keys are still lost on restart (§2).


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

- This sidecar is **new** relative to the CLI path. Treat it as an integration preview: correct wiring and fail-closed behaviour are covered by automated HTTP tests; it is **not** marketed as a finished, production-hardened appliance.
- Production key custody and AWS KMS for the sidecar are **follow-on** work (same open flank as CLI crypto / KMS provisioning in [BASELINE.md](../BASELINE.md)).
- For security evaluation of Solum overall, start from [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) and the current baseline tag.

**Contact:** [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)
