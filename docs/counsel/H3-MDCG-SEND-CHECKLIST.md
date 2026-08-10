# H3 MDCG — how to send external RA counsel review

**Audience:** Synaptic Four operator preparing **external** regulatory-affairs review
**Internal package (already signed):** [H3-MDCG-INTERNAL-REVIEW.md](H3-MDCG-INTERNAL-REVIEW.md)
**Portfolio:** Showcase [H3-PILOT-CHECKLIST.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H3-PILOT-CHECKLIST.md)

This is an **ops send checklist**, not legal or notified-body advice. External clearance is **open** and blocks **marketing** clinical / MDR claims — not H3 engineering exit.

---

## 1. Before you contact counsel / RA

- [ ] Confirm intended marketing claims (if any). Default: **no** “MDR certified”, “medical device”, or “clinical decision” language.
- [ ] Confirm eng owner can update docs / CONTRIBUTING if counsel requires wording changes.
- [ ] Identify counsel or RA consultant familiar with MDCG 2019-11 (and related) software qualification — EU focus for H3; Kenya is separate ([KENYA-K1-SEND-CHECKLIST.md](KENYA-K1-SEND-CHECKLIST.md)).

---

## 2. Package to send

Attach (or link with pinned commit SHA):

1. [H3-MDCG-INTERNAL-REVIEW.md](H3-MDCG-INTERNAL-REVIEW.md)
2. [PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) § intended purpose
3. [PARTNER-EHR-API.md](../customer/PARTNER-EHR-API.md)
4. [H3-CLINICAL-MODELLING.md](../H3-CLINICAL-MODELLING.md) — mapping honesty
5. CONTRIBUTING MDCG checklist excerpt
6. Optional: Showcase Path E+ fixtures note (digests ≠ certification)

Prefer a **zip** with fixed SHAs over moving `main` links.

---

## 3. Cover note (copy/adapt)

```
Subject: Synaptic Four Solum — H3 Track B MDCG / software qualification review

We operate an on-prem clinical-compliance sidecar (Solum). H3 adds an openEHR
CDR façade, FHIR subset store, migration dual-write, and subject bridge to
genomic object ids. Internal engineering review (attached) concludes these
surfaces do not perform diagnostic/therapeutic inference.

Please review whether our intended-purpose wording and customer docs remain
consistent with non-device / non-MDSW marketing under MDCG guidance for our
described features. We are not requesting notified-body certification in this
engagement unless separately scoped.

Reply format: short memo — confirm / risk / required wording changes.
Contact: contact@synapticfour.com
```

---

## 4. After counsel replies

| Outcome | Action |
|---------|--------|
| Confirms non-device posture for current surfaces | Record date in H3-MDCG-INTERNAL-REVIEW; allow cautious partner language already in PARTNER-EHR-API |
| Requires wording changes | Patch PRODUCT-DEFINITION / partner docs; re-run internal checklist |
| Flags a surface as MDSW risk | Freeze that route/claim; open eng issue before marketing |

**Still never claim:** MDR certification from this review alone.
