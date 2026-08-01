# Solum — Website product report

**Audience:** Visitors discovering Solum for the first time (website / sales orientation)
**Not for:** Security or legal teams already in a formal evaluation — use [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) and [customer/DEPLOYMENT-RUNBOOK.md](customer/DEPLOYMENT-RUNBOOK.md) instead.

**Truthfulness rule:** Same factual boundary as the customer security overview. This document changes **tone** (benefit-first, discovery-oriented), not **claims**. It is **not** legal advice, **not** a certification claim, and **not** a substitute for qualified regulatory-affairs review. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md))

**Authoritative product state:** What is finished in the current build is frozen in **[docs/BASELINE.md](BASELINE.md)** (versioned tag + verified commit). Prefer that file over marketing copy when exact capability boundaries matter.

---

## 1. What Solum is

**Solum is a compliance layer for clinical electronic health data** — it helps organisations **enforce** jurisdiction policy, **translate** interchange formats (FHIR first), and **produce evidence** of conforming processing and exchange, without becoming the system of record for your patient database. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §1; [architecture.md](architecture.md))

It is built for EU and African health markets as **equal core markets**: smaller and mid-size providers, clinics, labs, pharmacies, Health-Tech / HMIS / EHR vendors, and research-adjacent organisations that need EHDS- and data-protection-oriented technical controls without building a full compliance stack from scratch. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §2)

Solum is the clinical sibling to [Ferrum](https://github.com/SynapticFour/Ferrum) (genomic / GA4GH). Shared sovereignty philosophy — **separate brand, repository, and regulatory perimeter**. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §1; [README.md](../README.md))

---

## 2. Who Solum is for — two clear paths

### a) Solum as a standalone compliance layer

For organisations that need clinical compliance controls **without** adopting Ferrum.

Typical fit: clinics and practices, Health-Tech SMEs, HMIS/EHR vendors, labs and pharmacies that already store clinical data in their own systems and want policy enforcement, field encryption, consent/access evidence, and FHIR-oriented interchange support **beside** that stack. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §2; [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md) Mode A)

**What Standalone means in practice**

| | |
|---|---|
| **Storage** | Yours (bring your own). Solum does not replace your clinical database. |
| **Identity** | Your IdP / SMART-on-FHIR–shaped identity — no Ferrum Passport required for day-to-day operation. |
| **Ferrum dependency** | **None** required to run the product path. |

You keep operating your existing EHR/HMIS; Solum sits as the layer that refuses unsafe configurations at startup, encrypts selected clinical field categories, records consent and processing decisions, and exports a tamper-evident audit trail you can inspect. ([architecture.md](architecture.md); [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md))

### b) Solum as a Ferrum companion

For organisations that **already use or plan Ferrum** for genomic / research data infrastructure and need a **clinical** compliance layer next to it — not a second genome platform.

**Shared crypto base:** Both products use the **same Crypt4GH envelope format** — Ferrum for genomic objects, Solum for clinical field categories — so key material, tooling, and threat models can stay aligned across the portfolio. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md); [architecture.md](architecture.md); [CRYPTO.md](CRYPTO.md))

**Additive, never forced:** Ferrum integration in Solum is designed to be **additive** (existing Standalone APIs stay usable), **optional** (feature flags / separate constructors — never the default path), and validated against **both** operating modes. You do not have to adopt Ferrum storage or Ferrum auth to use Solum. ([INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md) — Leitprinzip)

Optional companion surfaces today include Crypt4GH format interop with Ferrum, optional mapping from Ferrum auth claims into Solum actors, and an optional storage backend feature — documented as reference/library paths, not as a mandatory turnkey bundle. ([BASELINE.md](BASELINE.md); [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md))

---

## 3. What Solum helps you do

Benefit language below maps 1:1 to implemented Stage‑1 capabilities. It does **not** claim legal compliance on your behalf.

### Jurisdiction rules as data, not hard-coded special cases

Policies live in **TOML jurisdiction profiles**. At startup Solum compares the active profile to runtime storage region, key-custody posture, mandatory audit events, and consent workflow — and **refuses to start** if they contradict. Changing market posture is primarily a **data** change, not a product fork per country. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §5–6; [profiles.md](profiles.md))

### Field encryption under customer-controlled keys

Selected clinical field categories are encrypted with **Crypt4GH**. Under customer-held custody, Solum **does not generate** regulated keypairs for you — you supply or register keys (or use a KMS/HSM you control). An **optional** AWS KMS path can protect Crypt4GH key seeds via an envelope model (feature-gated, default off). ([CRYPTO.md](CRYPTO.md); [BASELINE.md](BASELINE.md); [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) §3–4)

**Honest zero-knowledge path** (do not read this as “your data never leaves your infrastructure”):

> Solum does **not** claim cryptographic zero-knowledge for every operation. FHIR validation, access masking, and format transformation require processing. The realistic path is: (1) customer-held keys for data at rest, (2) confidential computing / TEE where processing must touch plaintext, (3) complete, customer-inspectable auditability as the accountability backbone. Encrypt/decrypt **touch plaintext briefly in process memory**. Full TEE isolation is a documented future direction, not current behaviour. ([architecture.md](architecture.md); [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) §3)

### Role-based access that fails closed

On the capability-checked path, grant, revoke, encrypt, and decrypt each require their **own** explicit permission. Missing or empty capabilities → **deny**, with an audited denial — no silent side effects, no `solum:*` superuser wildcard. The shipped CLI uses this path (`--capability`). Library callers that still use plain-string actor APIs bypass these checks — an acknowledged flank until they migrate. ([BASELINE.md](BASELINE.md); [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) §5)

### A tamper-evident audit trail you can verify

Deployments write a **durable, hash-chained** audit log. Operators can export a HELIOS-oriented evidence shape and verify chain integrity. **Live HELIOS signing is deferred and not productized** — export only. Stage 1 assumes a **single writer** to the audit file. ([architecture.md](architecture.md); [BASELINE.md](BASELINE.md); [helios.md](helios.md))

### FHIR-oriented interoperability (Stage 1 focus)

Stage 1 emphasises **FHIR** interchange, starting with an IPS-oriented **Patient Summary** binding. openEHR is present as a Stage‑2 scaffold. Broader EEHRxF priority categories (labs, discharge, imaging, prescriptions, …) remain on the roadmap. Structural IPS assumptions are **not** claimed as full IPS IG conformance pending specialist review. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §4; [BASELINE.md](BASELINE.md))

---

## 4. Why this matters now — regulatory context

### European Health Data Space (EHDS)

Solum’s EU orientation tracks **Regulation (EU) 2025/327** and EHDS Annex II–style security and logging expectations for EHR-related components, with technical preparation for primary-use interoperability (EEHRxF). Solum aims to support **technical readiness**; it does **not** declare that an operator is legally compliant. Operators must track applicable dates for certification, enforcement, and mandatory interoperability themselves. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §2, §4)

**ANNAHME, bitte prüfen:** Public discussion of EHDS implementation often includes **CE marking / conformity assessment obligations for EHR systems** and phased timelines. Exact dates and which obligations apply to which actor are **not frozen in this repository** (not listed in [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md) or [GTM-READINESS.md](GTM-READINESS.md)). Do not treat this report as a calendar of legal deadlines — verify against the Regulation and your counsel.

### African jurisdictions — equal core market, staged profiles

There is **no single African GDPR equivalent**. Nigeria, Kenya, South Africa, Egypt and others share principles but differ in thresholds, oversight, and timelines. Solum models that as **profile data**. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §2)

**Kenya** is the first African profile present in-tree — and it is explicitly a **DRAFT**: loadable, **not production-ready**, pending legal review (retention bases, audit retention figures, purpose catalogue, empty permitted transfer destinations, and national Health Data Bank obligations outside Solum’s scope). **Do not use the Kenya profile for a real deployment until those items are closed.** Nigeria NDPA– and South Africa POPIA–oriented profiles remain **planned**. ([BASELINE.md](BASELINE.md); [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) §7; [profiles.md](profiles.md))

---

## 5. Current maturity — said plainly

Solum is a **Stage‑1 product**: actively developed, on-premise first, installable today by **building from source** (no packaged binary release channel is documented in this repository). ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §5, §8; [BASELINE.md](BASELINE.md); [GTM-READINESS.md](GTM-READINESS.md))

What that means for someone discovering us on the website:

- Core Stage‑1 surfaces — jurisdiction profiles (EU present; Kenya draft), Crypt4GH field encryption with customer-held keys (optional AWS KMS library path), consent engine, capability-checked authorization (including CLI), durable hash-chained audit, FHIR Patient Summary — are **implemented and regression-tested**, with a public verification script and green CI on the frozen commit. ([BASELINE.md](BASELINE.md))
- Stage‑2 items (deeper EEHRxF categories, openEHR depth, SaaS operating model, additional jurisdiction packages) are **planned**, not shipped as finished product. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §8)
- Intended posture: **not a medical device** — manage, encrypt, log, translate, evidence; **never** interpret clinical data for diagnosis, therapy, or risk support. Classification still requires qualified regulatory review before go-to-market claims. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §3)
- There are **no named pilot customers or case studies** in this repository. We will not invent them.

We publish a versioned **[baseline](BASELINE.md)** that states exactly what passed local verification and CI — including accepted risks and what is explicitly out of scope. For a compliance product, that transparency is intentional: **you can see what is finished, what is draft, and what is still open**, instead of guessing from marketing slides.

---

## 6. Sovereignty philosophy

Solum’s design choices follow the same sovereignty line as Ferrum, applied to clinical data:

1. **Customer-held keys from the start** — not a retrofit after first deployment. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §5; [architecture.md](architecture.md))
2. **On-premise first** — Stage 1 targets operator-controlled deployments; SaaS is a prepared Stage‑2 path that must reuse the same key-custody, residency, and audit guarantees. ([architecture.md](architecture.md))
3. **Open standards** — Crypt4GH for field envelopes, FHIR for Stage‑1 interchange, alignment with GA4GH-oriented tooling via the Ferrum portfolio where genomic and clinical worlds meet — without locking interchange to a proprietary format. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §1, §4; [README.md](../README.md))
4. **Residency enforced at startup** — declaration without enforcement is documentation, not a guarantee. ([architecture.md](architecture.md))
5. **Honest about processing** — customer-held keys and inspectable auditability are the accountability backbone; we do not sell cryptographic zero-knowledge where the product must process plaintext. ([architecture.md](architecture.md))

---

## 7. Talk to us

Whether you are exploring Standalone fit for a clinic or EHR vendor, or Ferrum-Companion fit for a genomic + clinical stack:

- **[contact@synapticfour.com](mailto:contact@synapticfour.com)** · **[synapticfour.com](https://synapticfour.com)** ([README.md](../README.md))
- Ask for the **current baseline tag** and [docs/BASELINE.md](BASELINE.md) so any deeper conversation maps to a frozen commit.
- For security / legal evaluation packs: [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) and [customer/DEPLOYMENT-RUNBOOK.md](customer/DEPLOYMENT-RUNBOOK.md).
- Certified assessment / auditing is intended to be delivered by a **qualified external partner**, contractually separated — Synaptic Four does not present itself as the certified auditor. ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) §9)

---

*Working title **Solum** — final brand name may change. Built by [Synaptic Four](https://synapticfour.com).*
