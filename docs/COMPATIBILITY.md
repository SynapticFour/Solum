# Compatibility & deprecation policy

**Product:** Solum
**Status:** 2026-08-12 · org level-up **D4**
**Audience:** operators pinning Stage-1 / Track B pilots

Not [LICENSE-COMPATIBILITY.md](../LICENSE-COMPATIBILITY.md) (crate license policy).

---

## Versioning

- **SemVer** GitHub Release tags (`vX.Y.Z`) for product CLI/binaries ([RELEASING.md](../RELEASING.md)).
- Stage-1 baseline tags (`stage1-baseline-*`) are engineering freeze markers — pin explicitly when used.
- **MAJOR** may change sidecar flags, audit export schema versions, or remove soft-fail demo paths.

## Contracts we try to keep stable within MAJOR

| Contract | Notes |
|----------|-------|
| Consent / authz fail-closed behaviour | Behavioural; config profiles still matter |
| Audit export shape `solum-audit-helios-v1` / chain-v1 | Version the `format` field; consumers must tolerate additive fields |
| Subject-link API | `/v1/cdr/subject-link` — additive fields OK |
| Crypt4GH envelope family with Ferrum | Shared format; key custody remains site-owned |

## Deprecation

Document in CHANGELOG / BASELINE; keep deprecated flags at least one minor unless security requires otherwise.

## BUSL Change Date

BUSL-1.1 → **Apache-2.0** four years from each version’s release ([LICENSE](../LICENSE), [LICENSE-OPTIONS.md](LICENSE-OPTIONS.md)).

## Operator guidance

| Do | Don't |
|----|-------|
| Pin Solum tag/SHA in Showcase `PINNED_VERSIONS.txt` | Mix Track B EHRbase image floats with untested sidecar |
| Run Solum-Demo smokes after upgrade | Claim EHDS/MDR compliance from a version bump |
| Backup JSONL + EHRbase together | Restore CDR without Solum stores |

## Support

[Showcase support-tiers](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/for-customers/support-tiers.md).
