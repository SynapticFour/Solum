# Priorities (post IPS Validator Success — 2026-08-11)

Living engineering order after the proof-path harden landed on `main`
(`ae1592b` and docs follow-ups). Claims we already make must stay backed by
[CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md). Do **not** start ISiK/TI
adapter work without an explicit pilot Go ([DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md)).

## P0 — keep the trail honest

| Item | Why | Proof / exit |
|------|-----|--------------|
| Re-run `./scripts/verify.sh` after material `main` merges; pin **Verified commit** in [BASELINE.md](BASELINE.md) | Sales/eval freeze drift | `All baseline checks passed` + SHA update |
| Keep [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) in sync when adding claims | Prevent orphan marketing lines | Every new allowed claim gets a row + command |
| CI green on `main` after push | Same bar as verify | GitHub Actions CI + Secret Scan |

## P1 — close remaining Stage‑1 flanks (product trust)

| Item | Why | Notes |
|------|-----|-------|
| Legacy `&str` encrypt/decrypt: deprecate or document hard for integrators | Capability + consent bypass still exists on library path | CLI/`*_as` already gated; migration plan + warnings |
| Passport `SolumActor` mapping tests | Jwt path tested; Passport untested ([BASELINE.md](BASELINE.md)) | Add fixtures mirroring JWT coverage |
| KMS `EncryptionContext` / AAD binding | Optional AWS path honesty gap | Feature-gated; not blocking on-prem default |
| Wire Patient Summary encrypt/decrypt through `Deployment` + FileAuditStore | FHIR crypto today is crate-local; audit story incomplete for typed path | Still **no** product `solum fhir` CLI required — library/example OK |
| Optional: thin `solum fhir export-ips` CLI wrapping the example | Operators ask for one binary | Only if demos keep needing the example crate |

## P2 — geography / counsel / evidence portfolio

| Item | Why | Notes |
|------|-----|-------|
| Kenya **real counsel** send + named site (H4 K1/K3) | Profile is provisional after non-counsel Vorprüfung only | Empty `permitted_destinations` stays until TIA |
| Promote Nigeria / SA from `config/profiles/planned/` only after counsel | Scaffolds exist; must not become accidental production profiles | Checklist in `planned/README.md` |
| Live HELIOS signing bridge | Export envelope only today | Blocked on HELIOS release + custody story ([helios.md](helios.md)) |
| H5 managed custody / TEE — only if commercial SaaS path opens | Docs exist; not a launch | [H5-KEY-CUSTODY-MANAGED.md](H5-KEY-CUSTODY-MANAGED.md) |

## P3 — interchange depth (pilot-shaped)

| Item | Why | Notes |
|------|-----|-------|
| IPS remaining `ANNAHME`s (terminology, MedicationRequest, author Reference, MII URL) | Validator Success ≠ clinical IG completeness | Drive from a named IPS/IG version |
| Migration Prefer / Cut-over rehearsal on a real partner store | Track B dual-write stub exists | [MIGRATION-CUTOVER-CHECKLIST.md](MIGRATION-CUTOVER-CHECKLIST.md) |
| DE / ISiK adapter spike | Only with paying or committed pilot Go | [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md) — **gated** |
| EEHRxF priority categories beyond Patient Summary | Roadmap stage 2 | labs / discharge / imaging / Rx |

## Explicitly skipped now

- Full ISiK / gematik / TI connector without pilot Go
- Turning planned Nigeria/SA TOMLs into loadable production profiles without counsel
- Claiming live HELIOS attestation from Solum
- Claiming IPS “certified” beyond the pinned HL7 package campaign

## Done recently (do not re-open)

- Deny B consent-gated `*_as` crypto + worked example
- Proof path docs + `verify.sh` §8
- IPS Bundle UUID / LOINC / ait-1 / narratives → Validator Success
- Kenya / HELIOS / “no `solum fhir` CLI” honesty alignment
- Nigeria/SA **planned/** scaffolds (not auto-loaded)
