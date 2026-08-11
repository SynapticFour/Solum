# Migration cut-over checklist (H3.2)

Operator checklist for strangler stages in [MIGRATION-STRANGLER.md](MIGRATION-STRANGLER.md).
**Not** a certification claim.

## Stage 1 — Wrap (Track A)

- [ ] Sidecar consent / Crypt4GH / audit running with CustomerHeld keys
- [ ] Legacy EHR remains system of record
- [ ] H1/H2 pilot checklists signed where applicable

## Stage 2 — Mirror (H3.2)

- [ ] Track B EHRbase compose healthy ([H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md))
- [ ] Pinned OPT uploaded via `POST /v1/cdr/template`
- [ ] Batch inventory: `solum migrate fhir-import --bundle … --out inventory.jsonl`
- [ ] Import each inventory row via sidecar `POST /v1/fhir/{type}` (idempotent by resource id)
- [ ] Dual-write failures land in dead-letter JSONL — **never silent drop**
  - Live webhook: sidecar `POST /v1/migrate/dual-write` → `201` ok / `202` + dead-letter on mirror failure (`--dual-write-dead-letter` / `SOLUM_DUAL_WRITE_DEAD_LETTER`)
  - Offline/sim: `solum migrate dual-write-stub`
- [ ] Audit contains `cdr.fhir.created` / `cdr.dual_write.ok` / `cdr.dual_write.dead_lettered` (operator-visible)
- [ ] Round-trip verify N patients; rollback = disable dual-write + stop imports
- [ ] Clinical modelling honesty reviewed ([H3-CLINICAL-MODELLING.md](H3-CLINICAL-MODELLING.md)) — co-create, not full FHIR↔OPT map

**Offline tooling rehearsal (no Docker / no partner):** `./scripts/migration-rehearsal-dry-run.sh` exercises `fhir export-ips` + `migrate fhir-import` + `dual-write-stub` and prints the live-site checklist items still required.

## Stage 3 — Prefer (H3+)

- [ ] Partner UI / reports read Solum FHIR or AQL first for covered domains
- [ ] Domain list “Solum-preferred” documented
- [ ] Latency/error SLOs agreed with site

## Stage 4 — Cut-over (site-dependent; H3+/H4)

- [ ] Legal/ops sign-off for jurisdiction profile (Kenya: counsel pack)
- [ ] CDR backup/restore proven ([H3-EHRBASE-BACKUP.md](H3-EHRBASE-BACKUP.md))
- [ ] Legacy set read-only or decommissioned for covered domains
- [ ] Subject bridge links genomic DRS ids ([ADR 0003](adr/0003-subject-bridge.md))
- [ ] Reverse sync frozen
