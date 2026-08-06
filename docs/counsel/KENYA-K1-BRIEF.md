# Kenya jurisdiction pack — counsel brief (K1)

**Status:** Ready for external legal review
**Date:** 2026-08-06
**Profile:** `Solum/config/profiles/kenya-dpa.toml` (**DRAFT** — not for production SoR)
**Portfolio context:** [H4 geography decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md)

This brief is for a **qualified counsel / data-protection advisor** familiar with Kenya DPA 2019, Digital Health Act 2023, and ODPC guidance. It is **not** legal advice from Synaptic Four.

---

## 1. Ask to counsel

Please review the draft Solum jurisdiction profile and answer the **open items** in §3 so engineering can either:

- mark the profile **PRODUCTION-ready** (with documented assumptions), or
- keep **DRAFT** and list blocking conditions.

Synaptic Four will **not** claim ODPC registration, certification, or “Kenya-compliant EHR” from software alone.

---

## 2. What Solum does / does not do

| Does | Does not |
|------|----------|
| Enforce residency / key-custody / consent purposes / audit events declared in TOML at startup | Replace institutional DPA registration with ODPC |
| Crypt4GH field encryption with customer-held keys | Act as Health Data Bank submission system |
| Sidecar beside existing HMIS/EHR (Track A); optional future openEHR CDR (Track B) | Provide a full hospital EHR UI |
| Fail closed when runtime contradicts profile | Guarantee cross-border legality without case-by-case ODPC analysis |

Primary statutes cited in the profile header: DPA 2019; Digital Health Act 2023; ODPC Guidance Note on Processing of Health Data (2024); ODPC Guidance Note on Cross-border Data Transfers (2026).

---

## 3. Open items (need counsel answers)

### 3.1 Retention — clinical records

| Current draft | Conflict / question |
|---------------|---------------------|
| `default_retention_days = 7300` (20 years) | Digital Health Act s.25 vs DPA s.39 for **private** deployments — which floor/ceiling applies to a private lab/clinic pilot using Solum on-prem? |

**Ask:** Recommend a single retention table (clinical vs audit) for (a) private facility, (b) public facility, if different.

### 3.2 Retention — audit logs

| Current draft | Question |
|---------------|----------|
| `audit_log_retention_days` set in TOML | No ODPC-specified figure found in engineering research |

**Ask:** Minimum / recommended audit-log retention for health data processing systems under DPA + Digital Health Act.

### 3.3 Required purposes catalogue

| Current draft | Question |
|---------------|----------|
| `required_purposes` in profile (guidance-directed) | Not a statutory closed list |

**Ask:** Is the draft catalogue acceptable for pilots? What purposes must be present / forbidden for primary care vs research secondary use?

### 3.4 Cross-border transfer

| Current draft | Question |
|---------------|----------|
| `[transfer]` mechanisms partially modelled; `permitted_destinations` often empty → **fail-closed** | ODPC case-by-case guidance |

**Ask:** For an on-prem Kenya site that only stores in `KE` and never transfers abroad: is empty destinations + fail-closed correct? For EU research collaboration (Ferrum secondary use / HDAB-style): what destinations/mechanisms may we list without over-claiming?

### 3.5 National Health Data Bank

**Ask:** Confirm Solum’s explicit **non-goal** (no automatic national HDB submission) is appropriate for a private pilot, and what disclosure obligations remain on the **operator**.

### 3.6 Offline / Edge (Raspberry Pi)

**Ask:** Any additional constraints for offline capture with later sync to a Kenya hub (residency during sync, consent withdrawal while offline)?

---

## 4. Materials to attach for counsel

1. `config/profiles/kenya-dpa.toml` (full file)
2. `docs/profiles.md` § Kenya draft
3. `docs/PRODUCT-DEFINITION.md` § markets + MDCG posture
4. This brief

Optional: Showcase H1 pilot checklist (EU pilot path) for comparison.

---

## 5. Engineering commitments after counsel reply

| If counsel says… | We will… |
|------------------|----------|
| Retention X / Y | Update TOML + tests; document in `regulatory.notes` |
| Destinations list | Populate `permitted_destinations` or keep empty with runbook |
| Still insufficient | Keep STATUS DRAFT; block H4 Stage-4 cut-over |
| Approved with assumptions | Flip STATUS to PRODUCTION candidate + Showcase H4 sign-off |

---

## 6. Contact for Synaptic Four

- Product: Solum (clinical compliance / optional CDR)
- Email: contact@synapticfour.com
- Eng owner of profile schema: Solum `crates/profiles`

**Counsel name / firm / date received:** _______________________
