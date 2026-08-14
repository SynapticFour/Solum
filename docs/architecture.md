# Architecture

```
                    ┌─────────────────────────────────────┐
                    │            solum-core               │
                    │  startup validation · orchestration │
                    └───────────────┬─────────────────────┘
     ┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
     ▼          ▼          ▼          ▼          ▼          ▼          ▼          ▼
solum-profiles  crypto   fhir     openehr    audit    consent   identity  auth-verify
                                             sidecar (HTTP) wraps Deployment `*_as`
```

Solum is a **compliance layer** (Track A default): policy, interchange, evidence. Optional **Track B** fronts EHRbase as an openEHR CDR for partner APIs — not a Synaptic Four hospital EHR UI. See [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [adr/0002-cdr-engine-ehrbase.md](adr/0002-cdr-engine-ehrbase.md).

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

Declaration without enforcement is documentation, not a guarantee. `solum-profiles::validate_startup` compares the active jurisdiction profile to runtime storage region, key custody, mandatory audit events, and consent workflow. On contradiction the process **refuses to start**. Pilot CLI/sidecar additionally require an explicit `SOLUM_STORAGE_REGION` (operator attestation; EU/EEA refuses a contradictory `AWS_REGION`). This is not a cryptographic proof the host is in that region.

### Ferrum-core pinned, not duplicated

Same pattern as [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit): pin revision in `crates/crypto/Cargo.toml` and `config/ci/ferrum-revision.txt`. Product-specific FHIR/openEHR, EHDS workflows, and consent logic stay in this repository. Do not patch Ferrum for Solum-only needs without an explicit upstream product decision.

### Rust

Chosen for consistency with Ferrum-core and direct reuse of existing Rust building blocks.

## Crates

| Crate | Role |
|-------|------|
| `solum-core` | Product orchestration + `solum` CLI (`check`, `consent`, `crypto`, `audit`, `fhir`) |
| `solum-profiles` | Load/validate jurisdiction TOML profiles |
| `solum-crypto` | Crypt4GH field encryption + key custody; pins `ferrum-core` |
| `solum-fhir` | FHIR adapter (stage 1 focus) |
| `solum-openehr` | openEHR / EHRbase client (Track B) |
| `solum-audit` | Audit events; `FileAuditStore` persists a hash-chained, tamper-evident log + HELIOS-oriented JSON export |
| `solum-consent` | Grant/revoke consent per `(subject, purpose)`; purpose validated against the active profile; full history persisted |
| `solum-identity` | `SolumActor`, capability constants, fail-closed `require_capability` |
| `solum-auth-verify` | JWT/JWKS verification for sidecar org-IAM |
| `solum-sidecar` | HTTP façade over `Deployment` `*_as` (Track A + optional Track B CDR) |

`solum-core::Deployment` bundles a validated profile with its `FileAuditStore` and `ConsentStore` so consent decisions and their audit trail cannot drift apart — see its rustdoc in `crates/core/src/lib.rs`.

## FHIR/MII-Grenze zu Ferrum

Arbeitsteilung zwischen Ferrums struktureller FHIR-Validierung und Solums Jurisdiktions-Compliance (Sprint 3 — siehe [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md)). Dieser Abschnitt beschreibt eine **Absicht und Grenze**, keine getestete Integration: Solum ruft `ferrum-mii-connect` heute **nicht** auf.

- **`ferrum-mii-connect`** (Ferrum-Repo, nicht in diesem Workspace) prüft strukturelle FHIR-Konformität gegen die deutschen MII-Kerndatensatz-Profile (17 Module: Person, Encounter, Consent, Diagnose, Labor, Medikation, Onkologie, Pathologie, molekulare Berichte, Bildgebung, ICU, Biobank, Research Study u.a.). Ferrum positioniert das explizit als *"technical conformance, not legal advice about regulatory compliance"* — also **keine** rechtliche Compliance-Aussage.
- **`solum-fhir`** bleibt davon unabhängig nutzbar: IPS-orientierte Patient Summary, EHDS-fokussiert. Standalone-Betrieb (Mode A) hat **keine** Abhängigkeit von `ferrum-mii-connect`.
- **Ferrum-Companion-Modus (Mode B):** Wenn ein Feld bereits durch `ferrum-mii-connect` strukturell validiert wurde, kann Solum das über einen einfachen String-Verweis (`PatientSummary.mii_validation_ref`) referenzieren, statt eine zweite FHIR-Validierung parallel zu bauen. Arbeitsteilung: **Ferrum = Struktur-Konformität**, **Solum = Jurisdiktions-Compliance** (Verschlüsselungskategorie, Consent-Zweck, Audit). Der Verweis ist bewusst kein typisiertes Report-Objekt — die konkrete Report-API von `ferrum-mii-connect` ist hier nicht angenommen.
- **Nicht-Ziel dieses Sprints:** Live-Aufruf von `ferrum-mii-connect`, Übernahme von Report-Formaten, oder eine Solum-eigene MII-Profil-Engine.

## Related

- [Product definition](PRODUCT-DEFINITION.md)
- [Roadmap](roadmap.md)
- [Profiles](profiles.md)
- [HELIOS](helios.md)
- [Integration roadmap](INTEGRATION-ROADMAP.md)
