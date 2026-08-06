# Kenya K1 — how to send the counsel brief

**Audience:** Synaptic Four operator / founder preparing external legal review
**Brief:** [KENYA-K1-BRIEF.md](KENYA-K1-BRIEF.md)
**Profile:** `config/profiles/kenya-dpa.toml` (**PROVISIONAL-PRODUCTION-CANDIDATE** — real counsel still required)
**Portfolio:** Showcase [H4-GEOGRAPHY-DECISION.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md)

This is an **ops send checklist**, not legal advice.

---

## 1. Before you contact counsel

- [ ] Confirm eng owner for profile schema (`crates/profiles`) can turn counsel answers into TOML + tests within a defined window.
- [ ] Confirm commercial intent: sandbox / evaluation only until **PRODUCTION** after real counsel — never promise “Kenya-compliant” from software alone. Attach [KENYA-K1-VORPRUEFUNG.md](KENYA-K1-VORPRUEFUNG.md) labelled as **non-counsel** prior art.
- [ ] Identify counsel: Kenya DPA 2019 + Digital Health Act 2023 + ODPC health/cross-border guidance (local counsel preferred).
- [ ] Agree engagement scope in writing: answers to brief §3 only (retention, audit logs, purposes, transfer, HDB, offline) — not a full product certification audit.

---

## 2. Package to send

Attach (or link with pinned commit SHA):

1. [KENYA-K1-BRIEF.md](KENYA-K1-BRIEF.md) — primary ask
2. [KENYA-K1-VORPRUEFUNG.md](KENYA-K1-VORPRUEFUNG.md) — engineering prior art (**not** counsel)
3. `config/profiles/kenya-dpa.toml` — full file
4. `docs/profiles.md` — Kenya section
5. `docs/PRODUCT-DEFINITION.md` — markets + MDCG posture
6. Optional: Showcase H1 pilot checklist (EU path) for comparison only

Prefer a **zip or single email** with fixed SHAs over “latest main” links that move.

---

## 3. Cover note (copy/adapt)

```
Subject: Synaptic Four Solum — Kenya jurisdiction pack (K1) counsel review

We are preparing an on-prem clinical-compliance sidecar (Solum) for a Kenya
jurisdiction profile that is currently PROVISIONAL-PRODUCTION-CANDIDATE after an
internal non-counsel Vorprüfung (attached — not legal advice). Please review the
brief and answer §3 so we can mark PRODUCTION (with documented assumptions) or
keep provisional/DRAFT with blocking conditions.

We will not claim ODPC registration, certification, or “Kenya-compliant EHR”
from software alone. Operator/institutional obligations remain with the site.

Reply format: short memo keyed to §3.1–§3.6 is enough.
Contact: contact@synapticfour.com
```

---

## 4. After counsel replies

| Outcome | Action |
|---------|--------|
| Concrete retention / destinations / purposes | Update TOML + tests; note assumptions in `regulatory.notes`; eng K2 (Showcase H4) |
| “Insufficient / need site facts” | Keep DRAFT; record blockers in brief + H4 decision; no Stage-4 cut-over |
| Approved with assumptions | Flip STATUS to PRODUCTION candidate; Showcase H4 sign-off fields; still no certification claim |

Fill **Counsel name / firm / date received** at the bottom of [KENYA-K1-BRIEF.md](KENYA-K1-BRIEF.md).

---

## 5. What engineering does *not* wait on

- H2 product work (KMS, observability) — independent
- H3 EHRbase spike — independent
- K2 technical tests for KE — can start fixtures in DRAFT mode; **do not** market as production until K1 closes
