# H3 MDCG internal review package (H3.6)

**Status:** Internal engineering review — **SIGNED** 2026-08-10
**Not:** external regulatory-affairs / notified-body clearance (separate engagement, like Kenya K1 counsel).

## Posture (unchanged)

Solum’s intended purpose is to manage, encrypt, log, translate, and evidence clinical data processing — **never** to interpret data for diagnosis, therapy, or risk support. Sources: [PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md) §3, [CONTRIBUTING.md](../../CONTRIBUTING.md), PR/issue templates.

## H3 surfaces reviewed

| Surface | Inference risk | Attestation |
|---------|----------------|-------------|
| openEHR CDR façade `/v1/cdr/*` | Persistence + query only | No scoring / CDS |
| AQL proxy | Allowlisted SELECT | No clinical rules engine |
| FHIR façade subset | Store + optional CDR link | No IPS clinical decision logic |
| Migration import / dual-write | Copy/mirror | No enrichment beyond mapping |
| Subject bridge | Identifier join | No phenotype interpretation |
| Partner API docs | Integration contract | Explicit non-EHR / non-device language |

## Checklist (engineering)

- [x] CONTRIBUTING MDCG question remains mandatory for features
- [x] No H3 route performs diagnostic/therapeutic inference
- [x] Customer docs state non-device posture ([PARTNER-EHR-API.md](../customer/PARTNER-EHR-API.md), SIDECAR-INTEGRATION)
- [x] Showcase Path E+ honesty: digests ≠ MDR certification
- [ ] External RA counsel review before **marketing** clinical claims (open — send pack: [H3-MDCG-SEND-CHECKLIST.md](H3-MDCG-SEND-CHECKLIST.md))

## Sign-off

| Field | Value |
|-------|-------|
| Reviewer | Synaptic Four eng |
| Date | 2026-08-10 |
| Bound | H3.0–H3.5 engineering surfaces listed above |
