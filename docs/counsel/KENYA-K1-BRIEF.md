# Kenya jurisdiction pack — counsel brief (K1)

**Status:** Ready for external legal review (**still required**)
**How to send:** [KENYA-K1-SEND-CHECKLIST.md](KENYA-K1-SEND-CHECKLIST.md)
**Engineering prior art (not counsel):** [KENYA-K1-VORPRUEFUNG.md](KENYA-K1-VORPRUEFUNG.md) — applied 2026-08-06; **not** legal advice
**Date:** 2026-08-06
**Profile:** `Solum/config/profiles/kenya-dpa.toml` (**PROVISIONAL-PRODUCTION-CANDIDATE** — not PRODUCTION / not patient SoR)
**Portfolio context:** [H4 geography decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md)

This brief is for a **qualified counsel / data-protection advisor** familiar with Kenya DPA 2019, Digital Health Act 2023, and ODPC guidance. It is **not** legal advice from Synaptic Four.

A **non-counsel Vorprüfung** already adjusted profile honesty (retention claims, purposes, transfer fail-closed). Counsel must still confirm, amend, or reject those assumptions before PRODUCTION.

---

## 1. Ask to counsel

Please review the provisional Solum jurisdiction profile and answer the **open items** in §3 so engineering can either:

- mark the profile **PRODUCTION-ready** (with documented assumptions), or
- keep **PROVISIONAL** / revert to **DRAFT** and list blocking conditions.

Synaptic Four will **not** claim ODPC registration, certification, or “Kenya-compliant EHR” from software alone.

---

## 2. What Solum does / does not do

| Does | Does not |
|------|----------|
| Enforce residency / key-custody / consent purposes / audit events declared in TOML at startup | Replace institutional DPA registration with ODPC |
| Crypt4GH field encryption with customer-held keys | Act as Health Data Bank submission system |
| Sidecar beside existing HMIS/EHR (Track A); optional Track B openEHR CDR (H3 engineering exit; partner APIs, not a hospital EHR UI) | Provide a full hospital EHR UI |
| Fail closed when runtime contradicts profile | Guarantee cross-border legality without case-by-case ODPC analysis |

Primary statutes cited in the profile header: DPA 2019; Digital Health Act 2023; ODPC Guidance Note on Processing of Health Data (2024); ODPC Guidance Note on Cross-border Data Transfers (2026).

---

## 3. Open items (need counsel answers)

Engineering defaults after Vorprüfung are noted below — please **confirm or correct**.

### 3.1 Retention — clinical records

| Current provisional | Conflict / question |
|---------------------|---------------------|
| `default_retention_days = 7300` as **conservative Digital Health Act–aligned default**, not “Kenya requires 20 years for all private deployments” | Private facility vs public / integrated-system floor |

**Ask:** Confirm or replace the dual table (public/integrated ≈ 20y orientation; private = documented operator policy ≥ legal/contractual minimum).

### 3.2 Retention — audit logs

| Current provisional | Question |
|---------------------|----------|
| `audit_log_retention_days = 7300` as **security evidence retention** (no ODPC figure claimed) | Minimum / recommended under DPA + Digital Health Act |

**Ask:** Confirm 7–10y / match-processing guidance or give a figure to encode.

### 3.3 Required purposes catalogue

| Current provisional | Question |
|---------------------|----------|
| `required_purposes` = care_provision, emergency_access, care_coordination; research etc. in `optional_purposes` only | Acceptable for primary care vs research secondary use? |

**Ask:** Confirm floor + optional split; name any missing / forbidden purposes.

### 3.4 Cross-border transfer

| Current provisional | Question |
|---------------------|----------|
| `permitted_destinations = []` fail-closed; mechanisms include safeguards / hdab_mediated / statutory_exception (pathways ≠ permits) | Kenya-only OK? EU research collaboration listing? |

**Ask:** Confirm empty destinations for KE-only. What destinations/mechanisms may be listed for EU research without over-claiming?

### 3.5 National Health Data Bank

**Ask:** Confirm non-goal (no automatic HDB submission) and operator disclosure obligations wording in profile notes.

### 3.6 Offline / Edge (Raspberry Pi)

Engineering policy drafted in `regulatory.notes` (cache region = KE; no master keys on Pi; customer_held; restrict non-emergency when consent unknown pending sync).

**Ask:** Confirm or amend residency-during-offline, revoke-while-offline, and key constraints.

---

## 4. Materials to attach for counsel

1. `config/profiles/kenya-dpa.toml` (full file)
2. [KENYA-K1-VORPRUEFUNG.md](KENYA-K1-VORPRUEFUNG.md) (engineering prior art — label as non-counsel)
3. `docs/profiles.md` § Kenya
4. `docs/PRODUCT-DEFINITION.md` § markets + MDCG posture
5. This brief

Optional: Showcase H1 pilot checklist (EU path) for comparison.

---

## 5. Engineering commitments after counsel reply

| If counsel says… | We will… |
|------------------|----------|
| Retention X / Y | Update TOML + tests; document in `regulatory.notes` |
| Destinations list | Populate `permitted_destinations` or keep empty with runbook |
| Still insufficient | Keep PROVISIONAL or DRAFT; block H4 Stage-4 cut-over |
| Approved with assumptions | Flip STATUS to PRODUCTION candidate + Showcase H4 sign-off |

---

## 6. Contact for Synaptic Four

- Product: Solum (clinical compliance / optional CDR)
- Email: contact@synapticfour.com
- Eng owner of profile schema: Solum `crates/profiles`

**Counsel name / firm / date received:** _______________________
**Vorprüfung applied (non-counsel):** 2026-08-06 — see [KENYA-K1-VORPRUEFUNG.md](KENYA-K1-VORPRUEFUNG.md)
