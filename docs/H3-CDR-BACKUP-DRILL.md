# H3 CDR backup drill checklist

**Audience:** sites running Solum Track B (EHRbase)
**Status:** 2026-08-12 · org level-up **D3**
**Authoritative procedure:** [H3-EHRBASE-BACKUP.md](H3-EHRBASE-BACKUP.md)

Use this checklist to **sign** a site drill. Attach dump size, image pins, and elapsed time.

---

## Pre-drill

- [ ] Named backup owner + offsite location recorded
- [ ] `VERSIONS` / image digests captured
- [ ] Writers identified (sidecar dual-write, FHIR posts)
- [ ] Disposable or staging stack preferred for first drill

## Backup

- [ ] `pg_dump` EHRbase Postgres (`-Fc`)
- [ ] Consent / audit / subject-link / FHIR JSONL copied
- [ ] Dual-write dead-letter copied (if present)
- [ ] Keys/profile custody location verified (not only DB)

## Restore

- [ ] Writers stopped
- [ ] Empty Postgres + **same** image pins
- [ ] `pg_restore --clean --if-exists`
- [ ] JSONL restored to configured paths
- [ ] Sidecar started with `--ehrbase-url`
- [ ] Smoke: template / FHIR id / subject-link
- [ ] `solum audit verify` OK

## Sign-off

| Field | Value |
|-------|-------|
| Site | |
| Operator | |
| Date (UTC) | |
| Dump size | |
| Elapsed | |
| Pins / digests | |
| Result | PASS / FAIL |
| Notes | |

### Synaptic Four reference

| Field | Value |
|-------|-------|
| Date | 2026-08-12 |
| Result | **Checklist published** — live Track B drill remains **site-owned**; eng validated procedure text against H3-EHRBASE-BACKUP. Run on Solum-Demo `up-h3` before first customer Track B pilot. |

---

## Related

- Customer DR pack: Showcase [disaster-recovery.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/for-customers/disaster-recovery.md)
- Solum IR: [INCIDENT_RESPONSE.md](INCIDENT_RESPONSE.md)
