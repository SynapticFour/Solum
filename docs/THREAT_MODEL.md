# Solum — Threat Model

**Status:** Living · customer-shareable
**Version:** 1.0 · 2026-08-12
**Audience:** Security reviewers, operators, procurement
**Related:** [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) · [CRYPTO.md](CRYPTO.md) · [BASELINE.md](BASELINE.md) · [INCIDENT_RESPONSE.md](INCIDENT_RESPONSE.md) (when present)

Not legal advice. Not MDR/EHDS certification. Track A (sidecar) and Track B (optional CDR) are scoped separately below.

---

## 1. Product in one line

Solum is a **clinical-data compliance layer**: jurisdiction profiles, fail-closed authorization, consent-gated Crypt4GH field crypto, tamper-evident audit, and optional openEHR/FHIR CDR façade. It sits **beside** an existing EHR/HMIS (Track A) or optionally persists via a partner CDR (Track B) — it is **not** a Synaptic Four hospital EHR UI.

---

## 2. Assets

| Asset | Sensitivity | Track |
|-------|-------------|-------|
| Clinical FHIR/openEHR payloads (incl. special-category data) | Critical | A/B |
| Consent records / purpose bindings | Critical | A/B |
| Crypt4GH key material (CustomerHeld or KMS-wrapped seed) | Critical | A/B |
| Audit hash chain / HELIOS-oriented export | High | A/B |
| Subject-link store (`solum_subject_id` ↔ DRS / Phenopacket) | High | A/B |
| Jurisdiction profile TOML | Medium | A/B |
| EHRbase compositions / DB (when Track B enabled) | Critical | B |
| Sidecar process memory during encrypt/decrypt | Critical (transient) | A/B |

---

## 3. Trust boundaries

```text
  EHR / HMIS / partner UI          Ferrum (optional)
           │                              │
           ▼                              ▼
    ┌────────────────────────────────────────────┐
    │  Solum sidecar (authz, consent, crypto,    │
    │  audit, FHIR façade, optional CDR client)  │
    └───────────────┬────────────────────────────┘
                    │
         ┌──────────┴──────────┐
         ▼                     ▼
   Consent/audit stores    EHRbase (Track B)
   CustomerHeld keys       Customer DB
```

| Boundary | Notes |
|----------|-------|
| Operator | Configures profiles, keys, IdP groups → capabilities |
| Sidecar process | **Plaintext briefly in memory** on encrypt/decrypt (documented; not TEE). HTTP **loopback only**; TLS is the operator reverse proxy. |
| Ferrum companion | Shared Crypt4GH envelope family; consent revoke can 403 DRS/WES when wired |
| AWS KMS (optional) | Protects seed at rest; unwrap still lands seed in process memory |

---

## 4. Adversaries (in scope)

| Adversary | Goal | Posture |
|-----------|------|---------|
| External attacker | Bypass authz / read clinical fields | Fail-closed authz; **loopback-only HTTP** (TLS at the reverse proxy); no anonymous write |
| Auth’d user without purpose/consent | Access out-of-purpose data | Consent gates; Deny B paths |
| Insider with host access | Dump keys / DB | CustomerHeld + host controls; IR |
| Profile misconfiguration | Soft-open residency/consent | Pilot profiles require `SOLUM_STORAGE_REGION` attestation; Kenya is EVALUATION-ONLY until counsel |
| Supply-chain compromise | Malicious crate | cargo-deny (CI target); dependency-review |
| Audit tampering | Hide access | Hash-chain export; operator must protect store |

### Out of scope

| Non-goal | Meaning |
|----------|---------|
| Clinical decision support / diagnosis | Explicit non-goal (MDCG/RA before marketing claims) |
| Guaranteeing EHDS/GDPR legal compliance | Technical readiness only |
| Full EHR product security (UI, scheduling, billing) | Not Solum |
| TEE isolating plaintext-in-process | Future sketch |
| Synaptic Four holding production clinical keys | Default CustomerHeld |

---

## 5. STRIDE summary

| STRIDE | Examples | Mitigations | Residual |
|--------|----------|-------------|----------|
| Spoofing | Fake OIDC / CAP mapping | **Org-IAM required** on pilot profiles; `capability[]` only on `dev-local`; issuer+audience checked | Mis-mapped groups |
| Tampering | Edit consent JSONL / audit | Permissions; hash chain | Host admin |
| Repudiation | Deny clinical access | Audit events; HELIOS-oriented export | Export not live-signed inside Solum |
| Info disclosure | Ciphertext without key; logs | Crypt4GH; field crypto | Plaintext in process; log content policy |
| DoS | Flood sidecar | Operator capacity / reverse proxy | Limited in-product limits |
| Elevation | Capability bypass bugs | Tests; fail-closed defaults | Patch + IR |

---

## 6. Track B (CDR) additional risks

When EHRbase / openEHR persistence is enabled:

- CDR DB compromise ≈ clinical SoR compromise for covered domains.
- Migration/dual-write increases attack surface (legacy + Solum).
- Backup/restore is **site-owned** (see H3 EHRbase backup docs).
- Subject bridge must not become an uncontrolled cross-org MPI.

---

## 7. Pilot acceptance checklist

1. CustomerHeld keys (no ephemeral keys in pilot profile).
2. Authn/authz on; capabilities mapped intentionally.
3. Consent revoke tested against Ferrum when co-deployed.
4. Audit export produced and stored off-box.
5. Jurisdiction profile status understood (Kenya **EVALUATION-ONLY** until counsel).
6. IR contact path documented.

---

## 8. Maintenance

Update when custody modes, consent teeth, or Track B persistence change. Keep aligned with [BASELINE.md](BASELINE.md).
