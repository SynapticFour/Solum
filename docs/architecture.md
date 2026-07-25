# Architecture

```
                    ┌─────────────────────────────────────┐
                    │            solum-core               │
                    │  startup validation · orchestration │
                    └───────────────┬─────────────────────┘
     ┌──────────────┬────────────┬───────────┬────────────┬─────────────┬──────────────┐
     ▼              ▼            ▼           ▼            ▼             ▼
solum-profiles  solum-crypto  solum-fhir  solum-openehr  solum-audit  solum-consent
(TOML juris-    (ferrum-core  (stage 1)   (stage 2       (hash-chained (grant/revoke,
 diction         git pin)                  scaffold)      file log,     purpose
 profiles)                                                HELIOS export) binding)
```

Solum is a **compliance layer**: it enforces policy, transforms interchange formats, and produces evidence. It is not the system of record for durable clinical storage.

## Principles

### On-premise first

Stage 1 targets operator-controlled deployments. A SaaS path may follow in stage 2 and must reuse the same key-custody, residency, and audit guarantees — not weaken them.

### Customer-held keys

Encryption posture assumes keys remain under customer control from day one (not a later retrofit). Shared types/config come from git-pinned [`ferrum-core`](ferrum.md); Crypt4GH field encryption, key providers, and custody checks live in `solum-crypto`.

Both Ferrum and Solum use **Crypt4GH** envelopes; Ferrum for genomic DRS objects, Solum for clinical field categories — same format, different product surfaces ([CRYPTO.md](CRYPTO.md)).

### Honest zero-knowledge path

Solum does **not** claim cryptographic zero-knowledge for every operation. FHIR validation, access masking, and format transformation require processing. The realistic path is:

1. customer-held keys for data at rest,
2. confidential computing / TEE where processing must touch plaintext,
3. complete, customer-inspectable auditability as the accountability backbone.

### Residency and profile enforcement

Declaration without enforcement is documentation, not a guarantee. `solum-profiles::validate_startup` compares the active jurisdiction profile to runtime storage region, key custody, mandatory audit events, and consent workflow. On contradiction the process **refuses to start**.

### Ferrum-core pinned, not duplicated

Same pattern as [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit): pin revision in `crates/crypto/Cargo.toml` and `config/ci/ferrum-revision.txt`. Product-specific FHIR/openEHR, EHDS workflows, and consent logic stay in this repository. Do not patch Ferrum for Solum-only needs without an explicit upstream product decision.

### Rust

Chosen for consistency with Ferrum-core and direct reuse of existing Rust building blocks.

## Crates

| Crate | Role |
|-------|------|
| `solum-core` | Product orchestration + `solum` CLI (`check`) |
| `solum-profiles` | Load/validate jurisdiction TOML profiles |
| `solum-crypto` | Crypt4GH field encryption + key custody; pins `ferrum-core` |
| `solum-fhir` | FHIR adapter (stage 1 focus) |
| `solum-openehr` | openEHR adapter (stage 2 scaffold) |
| `solum-audit` | Audit events; `FileAuditStore` persists a hash-chained, tamper-evident log + HELIOS-oriented JSON export |
| `solum-consent` | Grant/revoke consent per `(subject, purpose)`; purpose validated against the active profile; full history persisted |

`solum-core::Deployment` bundles a validated profile with its `FileAuditStore` and `ConsentStore` so consent decisions and their audit trail cannot drift apart — see its rustdoc in `crates/core/src/lib.rs`.

## Related

- [Product definition](PRODUCT-DEFINITION.md)
- [Roadmap](roadmap.md)
- [Profiles](profiles.md)
- [HELIOS](helios.md)
