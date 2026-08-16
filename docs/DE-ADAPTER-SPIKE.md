# DE adapter — narrow KIS mapping on main; full ISiK still gated

**On `main` (M3):** identifier / name / birthDate Patient adapter in `solum-fhir` (`to_kis_patient_adapter`). Tag `kis-patient-v0`. **No** `meta.profile`, **no** ISiK IG validation, **no** SMC-B / TI. Operator notes: [examples/de-adapter/README.md](../examples/de-adapter/README.md).

**Still gated:** claiming an ISiK / gematik / TI profile instance, validator campaigns against a named DE IG package, epa write, full ISiK module set.

**Gate owner (full IG):** product / pilot lead. **Depends on:** [DE-FHIR-GAP.md](DE-FHIR-GAP.md) remaining P1 rows and a named target IG version.

## Why the rest stays gated

Building an ISiK/TI-shaped **HIS connector** without a paying or committed pilot pulls Solum toward EHR gravity (auth, profiles, terminology) and away from the compliance-layer moat already proven in [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md). The narrow adapter is interchange with the KIS that already exists — not a hospital EHR.

## When Go is given — remaining spike scope

1. **One** use case only (e.g. ISiK Basis Patient **or** one document type — not both).
2. Mapper or alternate export path in `solum-fhir` (feature-flagged or example binary) that **stamps the pilot IG profile**.
3. Same validator probe path as [FHIR-VALIDATION.md](FHIR-VALIDATION.md), pointed at the **pilot’s** IG package.
4. Update [DE-FHIR-GAP.md](DE-FHIR-GAP.md) campaign log with pass/fail rows.
5. Claims in customer docs only as narrow as the spike (“exports profile X validated against package Y”) — still not “TI-zertifiziert”.

## Explicitly out (narrow adapter and full spike)

- SMC-B / Fachdienst authentication
- epa write path
- Full ISiK module set
- MDR classification changes
- Hospital EHR / HIS product

## Exit criteria (full IG spike)

| Criterion | Done when |
|-----------|-----------|
| Pilot Go | Written decision (issue / mail) naming IG + use case |
| Mapper | Green validator against pinned package for that use case |
| Docs | Gap table rows flipped `fail` → `pass` / `partial` with date |
| Non-Go | Full IG mapper stays off `main`; the KIS adapter remains unprofiled |
