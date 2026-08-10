# H4 — Offline consent / sync / residency policy (Kenya pack)

**Audience:** operators deploying Solum Track A + Ferrum Edge on Pi with a hub in Kenya
**Profile:** [`kenya-dpa.toml`](../config/profiles/kenya-dpa.toml) (PROVISIONAL — counsel still required for PRODUCTION)
**Portfolio:** Showcase [H4-PILOT-CHECKLIST.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-PILOT-CHECKLIST.md) · [H4-HUB-PI-ARCHITECTURE.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-HUB-PI-ARCHITECTURE.md)

This document **writes** the sync/residency policy that unblocks the H4 Pi no-go (“no offline cut-over without policy”). Field **reconcile wiring** at a named site remains K3.

---

## Roles

| Node | Allowed Solum surfaces | Forbidden |
|------|------------------------|-----------|
| **Pi / Edge** | Track A: consent grant/revoke/status, Crypt4GH field crypto, audit append, subject-link **cache** if needed | EHRbase / Track B CDR, dual-write webhook as SoR, master key material in clear |
| **Hub** | Track A + Track B (EHRbase), FHIR façade, migration dual-write, full subject-link store | Treating empty `permitted_destinations` as allow-all |

Ferrum: capture + local DRS on Pi; heavy WES / Demo stack on hub ([FIELD-GA4GH-DEMO-PI.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/FIELD-GA4GH-DEMO-PI.md)).

---

## Residency

1. `offline_cache_region` **must equal** deployment / profile region: **`KE`**.
2. Encrypted local cache only (Crypt4GH / disk encryption at site policy).
3. No master / HSM unwrap keys permanently resident on Pi — CustomerHeld key files on Pi are **site-risk**; prefer hub custody for long-lived keys; Pi holds only session or narrowly scoped material per field runbook.
4. Cross-border hub URL (outside KE) is a **transfer** — refuse unless counsel-filled `permitted_destinations` + TIA; default empty list = fail-closed.

---

## Consent while offline

| Situation | Policy |
|-----------|--------|
| Grant recorded locally | Append to consent store; sync to hub when linked |
| Revoke recorded locally | Prefer **restrict** non-emergency processing until hub reconcile confirms |
| `consent.unknown` / Solum unreachable | Ferrum Teeth / Solum checks stay **fail-closed** for bound DRS/WES (no silent allow) |
| Emergency purpose | Only if profile lists `emergency_access` and site SOP allows; still audit |

After sync: hub is authoritative for conflicting grant/revoke; dead-letter unresolved conflicts for operator review (never silent drop).

---

## Sync order (recommended)

1. Solum consent + audit slices (hub merge)
2. Subject-link upserts (`solum_subject_id` ↔ `ferrum_drs_id`)
3. Ferrum `ferrum sync push` / sneakernet export ([FIELD-SYNC-QUEUE.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/FIELD-SYNC-QUEUE.md))
4. Hub-only: FHIR/CDR mirror if site uses Track B

---

## Honesty

- Policy written ≠ ODPC approval.
- `kenya-dpa` stays PROVISIONAL until K1 counsel.
- Named-site MoU + field reconcile drills are K3.
