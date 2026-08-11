# H3 Worked Evidence — Track B (EHRbase)

**Status:** packaging + honesty for an existing smoke — not a new CDR feature.
**Claims allowed:** “Solum façade + EHRbase smoke produces retained HTTP/JSON evidence when the stack is up.”
**Claims forbidden:** MDR clearance, patient-summary OPT pin, Synaptic Four EHR UI, production Keycloak topology, TI/ISiK readiness.

## What this proves

Optional **Track B**: Solum fronts EHRbase (Apache 2.0) for partner APIs. The automated proof lives in **Solum-Demo**:

```bash
# sibling checkout recommended: ../Solum-Demo beside this repo
cd ../Solum-Demo
make up-h3
make smoke-h3
```

`scripts/smoke-h3.sh` already writes under `artifacts/smoke-h3/` (gitignored in the Demo):

| File | Content |
|------|---------|
| `result.txt` | `PASS h3` / `SKIP: …` / `FAIL: …` |
| `MANIFEST.txt` | Short proof-path pointer (Track B evidence) |
| `template.json` | Template upload response |
| `ehr.json` | Created EHR id |
| `composition.json` | Example composition commit |
| `fhir-patient.json` | FHIR Patient + subject-link path |
| `subject-link.json` | Join key read-back |
| `dual-write.json` | Migration dual-write stub |
| `aql.json` | Allowlisted AQL proxy |

Soft-skip (exit 0) when the stack is down, unless `SOLUM_DEMO_H3_REQUIRE=1`.

## Operator narrative (Solum repo)

Bring-up and curl path without Demo Makefile: [H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md).
Partner surface: [customer/PARTNER-EHR-API.md](customer/PARTNER-EHR-API.md).
Clinical modelling honesty: [H3-CLINICAL-MODELLING.md](H3-CLINICAL-MODELLING.md).

## Sample evidence (redacted shape)

Successful smoke responses look like:

```json
{ "ehr_id": "<uuid>" }
```

```json
{ "composition_uid": "<uid::version>" }
```

```json
{ "resourceType": "Patient", "id": "demo-h3-…" }
```

Do **not** commit live smoke dumps with site tokens; keep them under Demo `artifacts/` (ignored).

## Relation to Track A worked example

| Track | Proof | Docker |
|-------|-------|--------|
| A | [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) · `verify.sh` §8 | No |
| B | This doc · Demo `make smoke-h3` | Yes (EHRbase + sidecar) |

Stage-1 Demo dashboard remains ephemeral-key demo; H3 overlay does not change that story.
