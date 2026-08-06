# Kenya K1 — Vorprüfung (non-counsel engineering review)

**Date:** 2026-08-06
**Status of input:** **Not from qualified Kenya counsel.** Internal / provisional engineering review used to unblock profile honesty.
**Profile after apply:** `config/profiles/kenya-dpa.toml` → **PROVISIONAL-PRODUCTION-CANDIDATE**
**Still required:** Real external review via [KENYA-K1-BRIEF.md](KENYA-K1-BRIEF.md) + [KENYA-K1-SEND-CHECKLIST.md](KENYA-K1-SEND-CHECKLIST.md) before any **PRODUCTION** claim or live patient SoR.

Do **not** treat this document as legal advice or ODPC clearance.

---

## How this was used

Engineering applied the Vorprüfung recommendations to TOML + notes so the profile stops over-claiming (20-year mandate for all; research as required purpose; destinations as free pass). K1 legal closure remains **open** until counsel confirms or amends §3 items.

| Item | Vorprüfung outcome | Engineering action |
|------|--------------------|--------------------|
| 3.1 Retention clinical | 7300 OK as conservative default; not universal private mandate | Kept `7300`; rewrote notes / comments |
| 3.2 Audit retention | No ODPC figure; evidence retention OK | Kept `7300`; honesty notes |
| 3.3 Purposes | Research must not be default required | `required_purposes` = care floor; `optional_purposes` = research etc. |
| 3.4 Transfer | Empty destinations + fail-closed correct | Kept `[]`; added `hdab_mediated`; mechanisms ≠ permit |
| 3.5 HDB | Non-goal correct | Strengthened operator-obligation note |
| 3.6 Offline / Edge | Need residency / revoke / key policies | Documented as engineering policy in `regulatory.notes` (wiring = K2/K3) |

---

## Explicit claims ban (unchanged)

- No “Kenya requires 20 years retention for all private deployments”
- No “Solum is ODPC-registered / Kenya-compliant EHR”
- No automatic National Health Data Bank submission
- No `permitted_destinations = ["EU"]` without TIA + approval

---

## Next steps

1. Send [KENYA-K1-BRIEF.md](KENYA-K1-BRIEF.md) package to Kenya counsel (attach this Vorprüfung as *engineering prior art*, not as legal conclusion).
2. On counsel reply: amend TOML or keep provisional; only then consider PRODUCTION.
3. K2: Edge offline policy enforcement (`offline_cache_region`, consent reconcile) when field path is scheduled.
