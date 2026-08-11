# DE adapter spike — **pilot-gated** (Phase 4)

**Status:** **Do not implement** until an explicit pilot Go exists.
**Gate owner:** product / pilot lead (not “nice to have on main”).
**Depends on:** [DE-FHIR-GAP.md](DE-FHIR-GAP.md) priorities P1+ and a named target IG version.

## Why gated

Building an ISiK/TI-shaped adapter without a paying or committed pilot pulls Solum toward **connector / EHR gravity** (auth, profiles, terminology) and away from the compliance-layer moat already proven in [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md).

## When Go is given — minimal spike scope

1. **One** use case only (e.g. ISiK Basis Patient **or** one document type — not both).
2. Mapper or alternate export path in `solum-fhir` (feature-flagged or example binary).
3. Same validator probe path as [FHIR-VALIDATION.md](FHIR-VALIDATION.md), pointed at the **pilot’s** IG package.
4. Update [DE-FHIR-GAP.md](DE-FHIR-GAP.md) campaign log with pass/fail rows.
5. Claims in customer docs only as narrow as the spike (“exports profile X validated against package Y”) — still not “TI-zertifiziert”.

## Explicitly out of spike

- SMC-B / Fachdienst authentication
- epa write path
- Full ISiK module set
- MDR classification changes

## Exit criteria

| Criterion | Done when |
|-----------|-----------|
| Pilot Go | Written decision (issue / mail) naming IG + use case |
| Mapper | Green validator against pinned package for that use case |
| Docs | Gap table rows flipped `fail` → `pass` / `partial` with date |
| Non-Go | This file remains the stop sign; no adapter code on `main` |
