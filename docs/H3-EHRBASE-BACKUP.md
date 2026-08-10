# EHRbase backup / restore (H3 ops)

**Audience:** operators running Solum Track B (`ehrbase` + `ehrbase-v2-postgres`)
**Compose:** Solum-Demo `docker-compose.ehrbase.yml` / `docker-compose.ehrbase-sidecar.yml`
**Pins:** Solum [`VERSIONS`](../VERSIONS)

This is a **drill runbook**, not a managed backup product. Sites own retention, encryption-at-rest, and offsite copies.

---

## What must be backed up together

| Component | Why |
|-----------|-----|
| Postgres volume for EHRbase | SoR compositions / EHRs |
| Solum audit JSONL | Hash-chained evidence |
| Solum consent store | Track A decisions |
| FHIR façade JSONL | H3.1 façade (not inside EHRbase) |
| Subject-link JSONL | Clinical ↔ genomic join |
| Dual-write dead-letter JSONL | Failed mirrors (never drop) |
| Profile + keys custody | Startup validation / decrypt |

Restoring EHRbase alone without Solum stores breaks Path E+ evidence.

---

## Backup (dev-local compose)

Assume project `solum-h3` and Postgres service name `ehrdb` (adjust to your compose file):

```bash
# 1) Consistent Postgres dump (logical)
docker compose -f docker-compose.ehrbase.yml exec -T ehrdb \
  pg_dump -U ehrbase -Fc ehrbase > "ehrbase-$(date -u +%Y%m%dT%H%M%SZ).dump"

# 2) Copy Solum JSONL stores (paths from sidecar flags / env)
cp "$SOLUM_AUDIT" "./backup/audit.jsonl"
cp "$SOLUM_CONSENT_STORE" "./backup/consent.jsonl"
cp "${SOLUM_FHIR_STORE:-./fhir_store.jsonl}" "./backup/fhir_store.jsonl"
cp "${SOLUM_SUBJECT_LINK_STORE:-./subject_links.jsonl}" "./backup/subject_links.jsonl"
cp "${SOLUM_DUAL_WRITE_DEAD_LETTER:-./dual_write_dead_letter.jsonl}" "./backup/dual_write_dead_letter.jsonl" 2>/dev/null || true
```

Record image digests from `VERSIONS` next to the dump.

---

## Restore drill

1. Stop writers (sidecar dual-write webhooks + FHIR posts).
2. Recreate empty Postgres volume; start EHRbase stack on **same** image pins.
3. `pg_restore -U ehrbase -d ehrbase --clean --if-exists ehrbase-….dump`
4. Restore Solum JSONL files to configured paths; start sidecar with `--ehrbase-url`.
5. Smoke: `POST /v1/cdr/template` (idempotent), `GET` a known FHIR id, `GET /v1/cdr/subject-link/{id}`.
6. Verify audit chain: `solum audit verify --audit …`.

---

## Exit criteria for a site

- [ ] Named backup owner + offsite location
- [ ] Successful restore drill dated in site runbook (attach dump size + VERSIONS pins)
- [ ] Dual-write paused/resumed procedure documented with dead-letter drain
