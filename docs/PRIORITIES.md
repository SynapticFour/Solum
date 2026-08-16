# Priorities (living — after P0–P3 engineering pass 2026-08-11)

Claims we make must stay backed by [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md).
Do **not** start a full ISiK/TI IG mapper without an explicit pilot Go
([DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md)). The unprofiled KIS Patient adapter
is already on `main`.

## Closed in this pass

| Was | Exit |
|-----|------|
| P0 BASELINE pin + claims trail + demo script | [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) · `./scripts/demo-claims-proof.sh` |
| P1 Legacy `&str` crypto/consent | `#[deprecated]` on `grant/revoke/encrypt/decrypt_field` (`&str`) |
| P1 Passport `SolumActor` tests | `crates/core/tests/solum_actor_auth.rs` |
| P1 KMS EncryptionContext | `seed_encryption_context` on wrap/unwrap; legacy empty context still loads |
| P1 Patient Summary via Deployment | `encrypt_patient_summary_as` / `decrypt_patient_summary_as` + audit |
| P1 Thin FHIR CLI | `solum fhir export-ips` |
| P2 Gate honesty (no unsafe promote) | Documented below; Nigeria/SA stay in `planned/` |
| P3 Author Reference ANNAHME | Organization entry + `author.reference` |
| P3 Migration Prefer/Cut-over dry rehearsal | `./scripts/migration-rehearsal-dry-run.sh` |

## Still open (external / pilot / stage-2)

### P2 — blocked on people / portfolio (not code this week)

| Item | Blocker | Next action |
|------|---------|-------------|
| Kenya **real counsel** + named site (H4 K1/K3) | External counsel + site | Operator send pack; keep `permitted_destinations = []` |
| Promote Nigeria / SA TOMLs into `config/profiles/` | Counsel | Follow `planned/README.md` checklist only after Go |
| Live HELIOS signing bridge | HELIOS release + custody | Keep export-envelope only ([helios.md](helios.md)) |
| H5 managed custody / TEE launch | Commercial SaaS decision | Docs only until Go ([H5-KEY-CUSTODY-MANAGED.md](H5-KEY-CUSTODY-MANAGED.md)) |

### P3 — remaining interchange depth

| Item | Status | Notes |
|------|--------|-------|
| IPS terminology binding (SNOMED etc.) | Open | Still ANNAHME — needs named IG + clinical codes |
| MedicationRequest path (vs MedicationStatement-only) | Open | IPS allows either; Statement-only remains stage-1 choice |
| Provisional MII extension URL | Open | Passthrough only |
| Live Prefer / Cut-over on partner store | Open | Dry rehearsal exists; live import needs Demo `smoke-h3` + site |
| DE / ISiK IG mapper | **Gated** | Narrow KIS adapter shipped; full IG still needs pilot Go |
| EEHRxF categories beyond Patient Summary | Stage 2 | labs / discharge / imaging / Rx |

### P0 ongoing hygiene

| Item | Cadence |
|------|---------|
| `./scripts/verify.sh` after material merges; pin [BASELINE.md](BASELINE.md) Verified commit | Every material merge |
| Keep CLAIMS map in sync when adding claims | With the claim |
| CI green on `main` | After each push |

## Explicitly still skipped

- Full ISiK / gematik / TI connector without pilot Go
- Loading planned Nigeria/SA as production profiles without counsel
- Claiming live HELIOS attestation from Solum
- Claiming IPS “certified” beyond the pinned HL7 package campaign
