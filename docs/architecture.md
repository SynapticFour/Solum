# Architecture

Solum is a **clinical-data compliance layer**: it enforces jurisdiction policy, translates interchange formats, and produces evidence of conforming processing. It is not a hospital EHR UI and does not interpret clinical data for diagnosis or therapy.

This document is the public architecture entry. It replaces the previous self-referential stub.

## Tracks

| Track | Role |
|-------|------|
| **A (default)** | Sidecar beside an existing clinical system of record. Policy, consent, audit, FHIR interchange, field encryption. No Solum-owned EHR store. |
| **B (optional)** | Partner-facing CDR façade in front of EHRbase (openEHR). FHIR subset, subject-link, migration helpers. Partners build UI on the APIs. |

Both tracks share the same policy engine, consent model, audit chain, and Crypt4GH field encryption. Track B is not a Synaptic Four hospital product.

## Crate map

| Crate | Responsibility |
|-------|----------------|
| `solum-core` | Profile load, residency checks, CLI `check`, orchestration types |
| `solum-profiles` | Jurisdiction TOML (`eu-ehds.toml` shipping; `kenya-dpa.toml` evaluation-only) |
| `solum-fhir` | FHIR interchange (stage-1 emphasis; IPS export is incomplete where marked) |
| `solum-openehr` | openEHR / EHRbase façade (Track B) |
| `solum-consent` | Grant / revoke per `(subject, purpose)` with history |
| `solum-audit` | Hash-chained, tamper-evident log; HELIOS-oriented **export** (signing is external) |
| `solum-crypto` | Crypt4GH field envelopes (X25519 header + ChaCha20-Poly1305); git-pinned `ferrum-core` types |
| `solum-identity` | Subject identifiers and org IAM bindings |
| `solum-auth-verify` | JWT / JWKS verification helpers |
| `solum-sidecar` | HTTP API: consent, audit, FHIR, subject-link, health |

The sidecar is the runtime process operators run (`solum-sidecar`). Libraries do not start a server by themselves.

## Startup enforcement

On boot the sidecar **refuses to run** if:

- the active jurisdiction profile cannot be loaded
- declared storage region contradicts the profile (`SOLUM_STORAGE_REGION` / profile allowed regions)
- key custody is `ephemeral_test` while the profile forbids it
- required audit or consent stores are not writable

Fail-closed is the default. Demo overlays (`dev-local.toml`) explicitly loosen this for laptop walkthroughs and are not a production posture.

## Keys and the honest zero-knowledge path {#honest-zero-knowledge-path}

Customer-held keys are the intended production custody (`KeyCustody::CustomerHeld`). Encrypt/decrypt **touch plaintext in process memory**. That is inherent to envelope crypto in this crate. Solum does **not** claim cryptographic zero-knowledge for validation, masking, or format transformation.

The realistic path is:

1. Customer-held keys for data at rest (Crypt4GH envelopes).
2. Confidential computing / TEE where processing must touch plaintext — **documented future direction, not current behaviour**.
3. A complete, customer-inspectable audit chain as the accountability backbone.

`OperatorHeld` is restricted. `EphemeralTest` is for tests and demos only.

## Persistence

- Track A: JSONL / file stores and operator-owned systems. Single-node. No HA in this repository.
- Track B: EHRbase + Postgres as the CDR. Backup/restore is operator-run (`docs/H3-EHRBASE-BACKUP.md`).
- Audit: append-only hash chain; tamper detection on read. HELIOS consumes an export file; this product does not live-sign to HELIOS.

## HTTP sidecar

The sidecar exposes versioned `/v1/*` routes (health, consent, audit, FHIR, subject-link). Production authz is capability- and consent-gated. Legacy `&str` APIs without a capability are deprecated.

Optional joins (not compiled into Solum as crates except git-pinned `ferrum-core`):

- Subject-link strings that a data-plane can store as metadata (`solum_subject` / `solum_purpose`).
- HELIOS export envelope `solum-audit-helios-chain-v1`.

## What this architecture is not

- Not a medical device.
- Not EHDS / DSGVO / HIPAA certification.
- Not a second genomics gateway.
- Not a multi-tenant SaaS control plane in Stage 1 (on-premise first).

Further product positioning: [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md). Evaluator snapshot: [FOR-EVALUATORS.md](FOR-EVALUATORS.md).
