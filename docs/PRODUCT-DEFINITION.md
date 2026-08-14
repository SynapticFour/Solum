# Solum — product definition (public)

Working title: **Solum**. Final brand name may change; this repository and package names use Solum until then.

This document is the in-repo product anchor for contributors and operators. It states positioning and design boundaries only. It is **not** legal advice, **not** a certification claim, and **not** a substitute for qualified regulatory-affairs review before market launch or public classification statements.

## 1. Positioning

| | [Ferrum](https://github.com/SynapticFour/Ferrum) | **Solum** |
|---|---|---|
| Domain | Genomic / -omic data under GA4GH | Clinical electronic health data |
| Role | Platform services for genomic exchange | **Compliance layer** *and* (optionally) **clinical data plane**: enforce, translate, evidence — and, when enabled, persist clinical content via open standards |
| Persistence | Data platform (operator-deployed) | **Track A (default):** does not replace the EHR — works with data wherever it already lives (sidecar). **Track B (H3 engineering exit):** optional openEHR CDR (EHRbase) + Solum façade, FHIR subset, migration helpers, subject bridge — partners build EHR UI on APIs |
| Docs | GA4GH / Crypt4GH / genomic EHDS notes | Links to Ferrum for GA4GH; does not duplicate it |

Both share a sovereignty philosophy (customer-held control, open standards, no lock-in to proprietary interchange formats) but are **separate brands, repositories, and regulatory perimeters**.

**Why Track B exists:** a pure EHDS/compliance shim is valuable early and vulnerable later as incumbents catch up. An optional openEHR-backed clinical store (APIs for others to build EHR UIs — **not** a full Synaptic Four EHR product) makes Solum a durable home for clinical data beside Ferrum genomics, with an explicit wrap → mirror → prefer → cut-over migration path. See the portfolio [coordinated roadmap](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/COORDINATED-PORTFOLIO-ROADMAP.md).

## 2. Markets

**EU EHDS is the shipping core.** Other jurisdictions are staged as profile data, not as equal production cores.

- **EU:** Smaller and mid-size health providers (practices, small clinics, labs, pharmacies) that must meet EHDS-related obligations without a large internal compliance/IT organisation. Legal frame: Regulation (EU) 2025/327. Operators must track applicable dates for certification, enforcement, and mandatory primary-use interoperability (EEHRxF) themselves; Solum aims to support technical readiness, not declare legal compliance.
- **Kenya:** Evaluation profile (`kenya-dpa.toml`) after a non-counsel engineering review. **Not** a production candidate, **not** ODPC-certified, **not** for live patient system-of-record until qualified Kenya counsel confirms.
- **Nigeria / South Africa:** Draft scaffolds under `config/profiles/planned/` — not auto-loaded, not counsel-reviewed.
- **Egypt:** Mentioned in older strategy notes; **no profile file exists** in this repository.

## 3. Regulatory boundary (MDCG) — non-negotiable

Solum’s intended posture is **not a medical device**: manage, encrypt, log, translate (e.g. FHIR ↔ openEHR), and evidence conforming processing — **never** interpret clinical data for diagnosis, therapy, or risk support.

- Process rule for every feature: see [CONTRIBUTING.md](../CONTRIBUTING.md).
- Classification of software under MDR/IVDR/AI Act depends on intended purpose and facts; public MDCG guidance informs design, but **qualified regulatory review is required before go-to-market claims**.
- If Solum were later classified because of a future feature set, that would **not** retroactively make Ferrum or HELIOS medical-device constituents; Solum would treat them as external components in its own technical documentation where applicable.

## 4. Standards

| Standard | Role in Solum | Notes |
|----------|---------------|--------|
| **FHIR** | Primary interchange (stage 1) | Open licensing; aligns with EEHRxF / EU Health Data API direction built on established IHE/HL7 practice while implementing acts evolve |
| **openEHR** | Structured clinical modelling (stage 2 depth) | Open tooling ecosystem; complementary to FHIR (model vs exchange) |
| **EHDS Annex II** | Security & logging floor for stage 1 | Binding essential requirements for EHR-related components independent of the final EEHRxF specification text |

Division of labour: **openEHR** for durable clinical modelling semantics; **FHIR** for exchange. Both are in the workspace from day one; stage 1 emphasises FHIR / EEHRxF readiness.

## 5. Architecture principles

Documented in depth in [architecture.md](architecture.md). Summary:

1. **On-premise first** (stage 1); SaaS is a prepared stage-2 path, not the initial delivery model.
2. **Customer-held keys from the start** — sovereignty via git-pinned `ferrum-core`; clinical fields encrypted with **Crypt4GH** in `solum-crypto` (same envelope as Ferrum — see [CRYPTO.md](CRYPTO.md)).
3. **Honest zero-knowledge path** — full cryptographic ZK is not claimed where validation, masking, or transform require processing; target is customer-held keys + confidential computing where appropriate + customer-visible auditability.
4. **Residency enforced at startup** — refuse to run if storage/key posture contradicts the active jurisdiction profile.
5. **Ferrum-core pinned, not forked** — Lab Kit pattern; product-specific clinical logic stays in this repo.
6. **Rust** — consistency with Ferrum-core and reuse of existing building blocks.

## 6. Jurisdiction profiles

Declarative TOML under [`config/profiles/`](../config/profiles/) (Ferrum Lab Kit pattern). Planned files (data only; no code branches per country):

| Profile file | Intent |
|--------------|--------|
| `eu-ehds.toml` | **Present** — EU EHDS Annex II orientation, EEHRxF preparation |
| `kenya-dpa.toml` | **Present** — EVALUATION-ONLY (non-counsel Vorprüfung; real counsel still required; **not** a production candidate) |
| `nigeria-ndpa.toml` | Planned — Nigeria NDPA-oriented controls |
| `south-africa-popia.toml` | Planned — POPIA-oriented controls |

Each profile declares encryption field categories, mandatory audit events, retention, allowed regions, and consent workflow; the runtime **enforces** them at startup. See [profiles.md](profiles.md).

## 7. HELIOS

[HELIOS](https://github.com/SynapticFour/HELIOS) produces signed, reproducible evidence (today oriented to pipeline/reproducibility contexts). Solum needs a related but distinct evidence class: **access / consent / processing-environment attestations** for clinical data.

Solum prepares stable export shapes (`solum-audit`) and intends to consume HELIOS where that fits, rather than duplicating a second evidence stack. Extension of HELIOS evidence types is an upstream portfolio request, not a copy of HELIOS into this repo. See [helios.md](helios.md).

## 8. Roadmap stages

See [roadmap.md](roadmap.md) and the portfolio [coordinated roadmap](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/COORDINATED-PORTFOLIO-ROADMAP.md).

- **Stage 1 (build now / Track A):** secure processing controls, consent/access-rights management, evidence generation hooks, profile system (EU profile + extensible schema), sidecar for legacy EHR wrap.
- **Stage 2 (Track B / H3):** **H3 engineering exit** — EHRbase CDR façade, FHIR subset + AQL proxy, migration inventory/dead-letter, subject bridge, partner API docs; deeper EEHRxF categories and external RA counsel remain follow-ons; SaaS *preparedness* without SaaS as default delivery.
- **Not planned as Solum:** a full hospital EHR UI, diagnostic/therapeutic decision support, or absorbing Ferrum’s genomic platform.

## 9. Certification partner model (concept)

Synaptic Four provides software and project coordination. **Certified assessment / auditing is delivered by a qualified external partner**, with liability for that work contractually separated. Synaptic Four does not present itself as the certified auditor.

Partner selection and commercial terms are **out of scope for this public repository**.

## 10. What this repo deliberately omits

Internal commercial materials (pricing, named partner shortlists, sales playbooks, trademark candidate lists) are kept out of the public tree. Contributors should not add them here.
