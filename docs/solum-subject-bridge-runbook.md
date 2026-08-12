# Operator runbook — subject bridge (mirror)

Canonical operator steps live in Ferrum:

→ [Ferrum `docs/solum-subject-bridge-runbook.md`](https://github.com/SynapticFour/Ferrum/blob/main/docs/solum-subject-bridge-runbook.md)

Solum API contract: [ADR 0003](adr/0003-subject-bridge.md) · [PARTNER-EHR-API.md](customer/PARTNER-EHR-API.md)

**Rule:** `solum_subject_id` (Solum) ≡ `solum_subject` (Ferrum metadata) ≡ optional BRA `phenopacket_id` join on the same subject-link row.
