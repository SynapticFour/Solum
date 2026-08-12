# Solum — Security & Compliance Overview

**Audience:** IT security and legal / compliance teams evaluating a pilot deployment
**Not for:** Software developers (see the repository `docs/` tree and [BASELINE.md](../BASELINE.md) for engineering detail)

**Authoritative product state:** This overview describes the behaviour frozen in the current Stage‑1 baseline. Concrete test counts, commit hashes, and accepted-risk wording change over time — treat **[docs/BASELINE.md](../BASELINE.md)** as the versioned source of truth for “what is in this build,” and treat this document as a customer-readable map of the same facts.

This document is **not** legal advice, **not** a certification claim, and **not** a substitute for qualified regulatory-affairs or data-protection review before go-to-market or public classification statements. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md))

**Also read:** [THREAT_MODEL.md](../THREAT_MODEL.md) (adversaries / residual risk) · [INCIDENT_RESPONSE.md](../INCIDENT_RESPONSE.md) (operator runbook)

---

## 1. What Solum is — and what it is not

### What it is

Solum is a **compliance layer** for clinical electronic health data: it **enforces** jurisdiction policy, **translates** interchange formats (FHIR first; openEHR / EHRbase on optional Track B), and **produces evidence** of conforming processing and exchange. Track A works with data wherever it already lives; Track B may persist via a partner-facing CDR façade — still **not** a Synaptic Four hospital EHR product. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §1; [architecture.md](../architecture.md); [H3-EHRBASE-SPIKE.md](../H3-EHRBASE-SPIKE.md))

Stage 1 delivery is **on-premise first**. A SaaS operating model is a prepared Stage‑2 path, not the initial delivery model. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §5; [architecture.md](../architecture.md))

### What it is not

- **Not a medical device (intended posture).** Solum’s intended posture is to manage, encrypt, log, translate (e.g. FHIR ↔ openEHR), and evidence conforming processing — **never** interpret clinical data for diagnosis, therapy, or risk support. Classification under MDR/IVDR/AI Act depends on intended purpose and facts; **qualified regulatory review is required before go-to-market claims**. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §3)
- **Not a hospital EHR UI / full clinical SoR product.** Track A does not replace primary clinical systems; optional Track B is a partner CDR façade, not Synaptic Four’s EHR application. ([architecture.md](../architecture.md); [PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md))
- **Not a hosted SaaS product in Stage 1.** ([architecture.md](../architecture.md))
- **Not a declaration of legal compliance.** Operators must track applicable dates for certification, enforcement, and mandatory primary-use interoperability themselves; Solum aims to support **technical readiness**, not declare legal compliance. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §2)

---

## 2. Operating models: Standalone vs. Ferrum-Companion

Solum supports two additive operating modes ([INTEGRATION-ROADMAP.md](../INTEGRATION-ROADMAP.md)):

| Mode | Storage | Authentication | Ferrum dependency |
|------|---------|-----------------|-------------------|
| **A — Standalone** | Customer-provided (bring your own) | Customer / SMART-on-FHIR–shaped identity | None required for day-to-day operation |
| **B — Ferrum-Companion** | Customer storage and/or optional Ferrum object storage | Optional Ferrum auth claims mapping | Optional, git-pinned shared crypto/auth building blocks |

Standalone remains fully usable without Ferrum platform services. Companion features are **additive** — existing Standalone paths stay available. ([INTEGRATION-ROADMAP.md](../INTEGRATION-ROADMAP.md); [architecture.md](../architecture.md) FHIR/MII boundary)

Both share a sovereignty philosophy with Ferrum (customer-held control, open standards) but are **separate brands, repositories, and regulatory perimeters**. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §1)

---

## 3. Encryption: Crypt4GH and customer-held keys

### Envelope

Clinical field categories are encrypted with **Crypt4GH** (X25519 header + ChaCha20-Poly1305 segments) — the **same envelope format** Ferrum uses for genomic objects, so keys, tooling, and threat models can stay aligned across the portfolio. ([CRYPTO.md](../CRYPTO.md); [architecture.md](../architecture.md))

### Customer-held custody (honest language)

Under customer-held key custody:

- Solum **never generates** Crypt4GH keypairs for regulated customer-held deployments. Keys are supplied / registered by the customer (or by a KMS/HSM the customer controls). ([CRYPTO.md](../CRYPTO.md); crypto module docs mirrored in [BASELINE.md](../BASELINE.md))

**Honest zero-knowledge path** (do not read this as “keys never leave your infrastructure in any form”):

> Solum does **not** claim cryptographic zero-knowledge for every operation. FHIR validation, access masking, and format transformation require processing. The realistic path is: (1) customer-held keys for data at rest, (2) confidential computing / TEE where processing must touch plaintext, (3) complete, customer-inspectable auditability as the accountability backbone. ([architecture.md](../architecture.md) — *Honest zero-knowledge path*)

> Encrypt/decrypt **touch plaintext in process memory**. Brief in-process plaintext during encrypt/decrypt is inherent to that path. Full confidential-computing isolation (TEE) is a documented **future direction**, not current behaviour. ([architecture.md](../architecture.md); KeyCustody documentation summarised in [BASELINE.md](../BASELINE.md))

### What this means in practice

Ciphertext at rest is protected under customer-controlled key material. When a field is encrypted or decrypted, plaintext exists **briefly inside the Solum process**. That is an intentional, documented property — not an accidental omission.

---

## 4. Key custody: on-prem default + optional cloud KMS

**Default and target:** on-premise (or customer VPC) **CustomerHeld** key files — CLI `--keypair`, sidecar `--keys-dir`. No cloud account is required. The same path works on bare metal, Hetzner, Azure, Alibaba, AWS, or a custom private cloud. ([CRYPTO.md](../CRYPTO.md); [BASELINE.md](../BASELINE.md))

### Optional AWS KMS adapter (not the product default)

AWS KMS is an **optional**, feature-gated custody path (default off). It is **not** a prerequisite for running Solum and does **not** make Solum AWS-only. ([BASELINE.md](../BASELINE.md))

**Technical constraint (from GTM‑2 research):** AWS KMS does not hold Crypt4GH’s native X25519 keys directly. Solum therefore uses an **envelope model**: KMS protects the Crypt4GH private-key seed at rest; Solum unwraps that seed briefly in process for the Crypt4GH operation.

**Operational caveats (current baseline):**

- Provisioning: library API **and** optional CLI/sidecar behind `--features aws-kms` (`wrap-seed`, `--wrapped-keypair`, `--wrapped-keys-dir`; rustc ≥ 1.94.1 for that feature). ([BASELINE.md](../BASELINE.md))
- Unwrapped seed material is held in ordinary process memory with **best-effort `ZeroizeOnDrop`** (not a TEE). ([BASELINE.md](../BASELINE.md))
- New KMS wraps bind EncryptionContext (`solum:purpose`, `solum:key_ref`); legacy files without context still unwrap. ([BASELINE.md](../BASELINE.md))
- CI covers mocked KMS behaviour, not live AWS accounts. ([BASELINE.md](../BASELINE.md))
- **Other clouds:** Azure Key Vault, Alibaba KMS, Hetzner-native secrets, and custom HSMs are **not** first-class Solum providers yet — use CustomerHeld files (or export material into CustomerHeld registration) until those adapters exist. ([CRYPTO.md](../CRYPTO.md))

Manual registration of customer-supplied key material (without AWS) remains the primary path for non-AWS and multi-cloud deployments. ([BASELINE.md](../BASELINE.md); [CRYPTO.md](../CRYPTO.md))

---

## 5. Access control: consent + capability-based authorization

### Consent engine

Consent is managed as grant / revoke decisions per **(subject, purpose)**, with purposes validated against the active jurisdiction profile, and a full history retained (EEHRxF-style individual rights orientation). Grant and revoke that go through the product orchestration path also write matching audit events in the same call, so consent state and audit trail do not silently drift apart under normal use. ([architecture.md](../architecture.md); [BASELINE.md](../BASELINE.md))

### Role / capability checks (GTM‑1)

On the structured-actor path (actor identity that carries **scopes / capabilities**), Solum checks **before** grant, revoke, encrypt, or decrypt whether the actor’s scopes contain the exact capability required for that operation. ([BASELINE.md](../BASELINE.md); [BASELINE.md](../BASELINE.md))

**Capability-based, fail-closed — practical meaning for operators:**

| Principle | What it means for you |
|-----------|------------------------|
| **Capability-based** | Each sensitive operation needs its own explicit permission string (e.g. grant consent ≠ revoke consent; encrypt ≠ decrypt). Having one capability does **not** imply another. |
| **Fail-closed** | If the required capability is missing — including when the actor has **empty** scopes — the operation is **denied**. There is no implicit “admin” or `solum:*` wildcard. |
| **Audited denials** | A denied attempt writes an `authorization.denied` audit event (failure outcome). The underlying consent or crypto side effect does **not** run. |
| **Exact match only** | Capabilities are compared as exact strings. Hierarchies / wildcards are **not** supported in this baseline. |

**Important asymmetry:** Older **library** call paths that identify the actor only as a plain text string **do not carry scopes and therefore do not enforce these checks**. That is intentional (legacy path) and remains an **open security flank** for any integrator that still calls those APIs. The shipped **CLI** uses the capability-checked path: pass `--capability` (repeatable); omit it → empty scopes → **fail-closed denial**. See §8 and [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md). ([BASELINE.md](../BASELINE.md))

---

## 6. Audit trail

Solum persists a **durable, hash-chained, tamper-evident** audit log for deployments. Operators can export a HELIOS-oriented JSON evidence shape and verify the chain integrity. ([architecture.md](../architecture.md); [BASELINE.md](../BASELINE.md); [helios.md](../helios.md))

**What this is:** customer-inspectable accountability for access, consent, crypto, and authorization-denial events prepared for external evidence tooling.

**What this is not:** live HELIOS CLI/API signing inside Solum. Export envelopes are prepared; **live HELIOS signing is deferred and not productized** — do not claim it in evaluations. ([helios.md](../helios.md); [roadmap.md](../roadmap.md))

**Operational limit:** Stage 1 assumes a **single writer** to the durable audit file. Multi-writer backends are Stage‑2 / out of this baseline. ([BASELINE.md](../BASELINE.md))

---

## 7. Jurisdiction profiles

Policies are **data files** (TOML), not hard-coded country branches. At startup Solum compares the active profile to runtime storage region, key-custody posture, mandatory audit events, and consent workflow — and **refuses to start** on contradiction. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §5–6; [profiles.md](../profiles.md))

| Profile | Status |
|---------|--------|
| EU EHDS–oriented profile (`eu-ehds`) | **Present** — production-track orientation for Stage 1 (Annex II–oriented controls). Still not a legal compliance certificate. |
| Kenya DPA / Digital Health Act profile | **Present as PROVISIONAL-PRODUCTION-CANDIDATE** after a **non-counsel** Vorprüfung — loadable but **not** production SoR / **not** ODPC-certified. Real Kenya counsel still required. Empty `permitted_destinations` → every concrete cross-border destination check **fails closed** until TIA + approval fill the list. ([profiles.md](../profiles.md); [BASELINE.md](../BASELINE.md)) |
| Nigeria NDPA–oriented | **DRAFT scaffold only** under `config/profiles/planned/` — not auto-loaded; not counsel-reviewed |
| South Africa POPIA–oriented | **DRAFT scaffold only** under `config/profiles/planned/` — not auto-loaded; not counsel-reviewed |

EU and African markets are equal core markets in product strategy; profile availability is staged as data. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §2)

---

## 8. Known limitations (do not skip)

The following are **accepted or open limitations** of the current baseline, restated for non-developer readers. Full engineering wording: [BASELINE.md](../BASELINE.md) — *Bewusst akzeptierte Risiken* and *Explizit außerhalb dieser Baseline*.

1. **Paid evaluations must use CustomerHeld `--keypair` (or library/KMS).** The CLI evaluation path is operator-supplied keypair files via `crypto keygen` + `--keypair`. Ephemeral keys (`--ephemeral`) require `SOLUM_ALLOW_EPHEMERAL=1` and `dev-local` (or another profile allowing `ephemeral_test`); pilot profiles refuse `EphemeralTest` custody. **Never describe ephemeral keys as a paid-evaluation custody option.** File keypairs hold private key bytes in plaintext JSON (0600 on Unix) — not an HSM. ([DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) §4)

2. **Legacy library actor paths skip authorization.** Callers that invoke grant/revoke/encrypt/decrypt with a plain actor string (no scopes) **bypass GTM‑1 capability checks entirely**. The shipped **CLI** does **not** use that path: it builds a structured actor from `--actor` + `--capability` and calls the checked APIs (omit `--capability` → fail-closed denial). Library integrators that still use the plain-string APIs remain on the unchecked flank. ([BASELINE.md](../BASELINE.md))

3. **No capability wildcards / hierarchies.** Exact string match only — e.g. no `solum:*` superuser scope. ([BASELINE.md](../BASELINE.md))

4. **IPS / FHIR Patient Summary structural assumptions are unchecked by a FHIR/IPS specialist.** Stage‑1 choices (document/section codes, empty-section encoding, MedicationStatement-only, no IPS terminology binding) are **not** claimed IPS IG conformance. Author now references an Organization entry. Subject-matter review is required before treating this as production interchange. ([BASELINE.md](../BASELINE.md))

5. **Best-effort zeroize-on-drop only** for key material in customer-held or AWS-KMS-backed providers (not a TEE / memory-dump proof). ([BASELINE.md](../BASELINE.md))

6. **Kenya profile is provisional, not production-closed.** Non-counsel Vorprüfung applied; qualified counsel still required before live SoR. ([BASELINE.md](../BASELINE.md); [profiles.md](../profiles.md))

7. **Kenya transfer destinations list is empty by design.** Listed transfer *mechanisms* are pathways only — `validate_transfer` rejects every concrete destination until counsel/TIA populate `permitted_destinations`. ([BASELINE.md](../BASELINE.md))

8. **AWS KMS path caveats:** EncryptionContext on new wraps; optional feature (CLI/sidecar); mocked tests only in CI; not HSM; seed unwrapped into process memory. ([BASELINE.md](../BASELINE.md))

9. **Optional object-storage backend** (Ferrum LocalStorage path) pulls a transitive cloud SDK even when only local storage is used; that feature stays default-off. CI coverage for the feature path is limited relative to the default build. ([BASELINE.md](../BASELINE.md))

10. **Standalone JWT/JWKS verification** is an independent implementation that can drift from Ferrum’s private verification behaviour; no live identity-broker test in CI (offline fixtures only). Passport-claim mapping for actors is implemented but not fully tested the way the JWT path is. ([BASELINE.md](../BASELINE.md))

11. **Known third-party advisory (RSA / Marvin Attack)** is tracked and intentionally ignored pending upstream migration away from RSA-based JWT paths — not introduced by Solum’s Crypt4GH field encryption itself. Details: [BASELINE.md](../BASELINE.md); [LICENSE-COMPATIBILITY.md](../../LICENSE-COMPATIBILITY.md).

12. **Single-writer audit store; no multi-instance audit backend** in Stage 1. ([BASELINE.md](../BASELINE.md))

13. **Binary install via GitHub Release is prepared but only after a verified SemVer `v*` tag.** Until then, install from source (see [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md) §1 and [RELEASING.md](../../RELEASING.md)).

---

## 9. Contact / next steps for security questions

- Product & security questions: [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com) ([README.md](../../README.md); [ECOSYSTEM.md](../ECOSYSTEM.md))
- Ask for the **current baseline tag** and [docs/BASELINE.md](../BASELINE.md) when starting a security review, so findings map to a frozen commit.
- Operational install steps: [DEPLOYMENT-RUNBOOK.md](DEPLOYMENT-RUNBOOK.md)
- Certified assessment / auditing is intended to be delivered by a **qualified external partner**, contractually separated — Synaptic Four does not present itself as the certified auditor. Partner selection is out of scope for this public repository. ([PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §9)

---

### Appendix — internal names (for auditors tracing claims)

| Customer term | Repository location (traceability only) |
|---------------|----------------------------------------|
| Product orchestration / CLI | `solum-core` |
| Jurisdiction profiles | `solum-profiles`, `config/profiles/` |
| Crypt4GH + key custody | `solum-crypto` (optional feature `aws-kms`) |
| Consent | `solum-consent` |
| Audit / HELIOS export shape | `solum-audit` |
| Actor + capabilities | `solum-identity` (`CAP_*`, `require_capability`) |
| FHIR Patient Summary | `solum-fhir` |
| Frozen state | [docs/BASELINE.md](../BASELINE.md), tag `stage1-baseline-gtm1-2026-07-28` |
