# H3.0 — EHRbase compose spike + Solum façade

**Status:** H3 Track B engineering exit available via Solum-Demo overlay + `make smoke-h3`
**Evidence packaging:** [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) (retained Demo `artifacts/smoke-h3/`)
**Not claimed:** MDR clearance, patient-summary OPT pin, Synaptic Four EHR UI, production topology
**Pins:** Stage-1 Demo uses Solum tag (see Demo `PINNED_VERSIONS.txt`); H3 sidecar builds **local** `../Solum`, not that tag.

Solum fronts **EHRbase** (Apache 2.0) as the Track B openEHR CDR. This document is the operator smoke path for **dev-local evaluation only**. Automate with Solum-Demo [`make smoke-h3`](https://github.com/SynapticFour/Solum-Demo/blob/main/Makefile) after `make up-h3`. Retained evidence: Demo `artifacts/smoke-h3/` — see [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md).

## Pins

See [`VERSIONS`](../VERSIONS):

| Key | Value |
|-----|--------|
| `EHRBASE_IMAGE` | `ehrbase/ehrbase:2.34.0` |
| `EHRBASE_POSTGRES_IMAGE` | `ehrbase/ehrbase-v2-postgres:16.2` |
| `SOLUM_H3_TEMPLATE_ID` | `minimal_observation.en.v1` |

Template fixture: [`crates/openehr/fixtures/minimal_observation.opt`](../crates/openehr/fixtures/minimal_observation.opt) (from EHRbase test resources; Apache 2.0).

## Honesty limits

- **Second runtime:** JVM EHRbase beside Solum — hub-class, **not** Pi / Edge Track B.
- **No Keycloak** in this spike — open local profile; not a production topology.
- **Not an EHR product** — APIs for others to build UIs; no diagnostic inference.
- Stage-1 Solum-Demo dashboard remains ephemeral-key demo; CDR overlay does not change that story.

## Bring-up (Solum-Demo overlay)

From the [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) checkout (sibling to Solum recommended):

```bash
# EHRbase + Postgres only (host port 8081 → container 8080)
docker compose -f docker-compose.ehrbase.yml up -d

# Optional: Sidecar with Track B façade (builds local ../Solum tree)
docker compose -f docker-compose.ehrbase.yml -f docker-compose.ehrbase-sidecar.yml --profile h3-sidecar up --build -d
```

Health:

- EHRbase welcome: `http://localhost:8081/ehrbase/`
- Management health (if exposed): `http://localhost:8081/ehrbase/management/health`
- Sidecar (when profile enabled): `http://localhost:8787` with `X-Solum-Sidecar-Token`

Stop / reset CDR volumes:

```bash
docker compose -f docker-compose.ehrbase.yml -f docker-compose.ehrbase-sidecar.yml --profile h3-sidecar down -v
```

## Smoke through Solum (not raw EHRbase alone)

With sidecar started with `--ehrbase-url http://ehrbase:8080/ehrbase` (compose) or `http://127.0.0.1:8081/ehrbase` (host):

```bash
TOKEN=solum-demo-local-token-not-for-production
BASE=http://127.0.0.1:8787

# 1) Ensure pinned template is loaded (idempotent upload)
curl -sS -X POST "$BASE/v1/cdr/template" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"actor":"practitioner/h3","capability":["solum:cdr:write"]}'

# 2) Create EHR
EHR=$(curl -sS -X POST "$BASE/v1/cdr/ehr" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"actor":"practitioner/h3","capability":["solum:cdr:write"]}')
echo "$EHR"
EHR_ID=$(echo "$EHR" | python3 -c "import sys,json; print(json.load(sys.stdin)['ehr_id'])")

# 3) Commit composition for pinned template (canonical example from EHRbase)
COMP=$(curl -sS -X POST "$BASE/v1/cdr/ehr/$EHR_ID/composition" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"actor\":\"practitioner/h3\",\"capability\":[\"solum:cdr:write\"],\"use_example\":true}")
echo "$COMP"
COMP_UID=$(echo "$COMP" | python3 -c "import sys,json; print(json.load(sys.stdin)['composition_uid'])")
ENC_UID=$(python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=''))" "$COMP_UID")

# 4) Read back (URL-encode composition uid — may contain `::`)
curl -sS "$BASE/v1/cdr/ehr/$EHR_ID/composition/$ENC_UID" \
  -H "X-Solum-Sidecar-Token: $TOKEN" \
  -G --data-urlencode "actor=practitioner/h3" \
  --data-urlencode "capability=solum:cdr:read"

# 5) Confirm audit contains façade write
curl -sS "$BASE/v1/audit/export" -H "X-Solum-Sidecar-Token: $TOKEN" | grep -E 'cdr\.(ehr\.created|composition\.committed)'
```

Expected audit event types on success: `cdr.ehr.created`, `cdr.composition.committed` (plus template upload if performed).

## Local cargo (without Demo compose sidecar)

```bash
# Terminal A — EHRbase stack (Demo overlay)
# Terminal B — from Solum repo:
export SOLUM_ALLOW_EPHEMERAL=1
cargo run -p solum-sidecar -- \
  --ephemeral \
  --profile config/profiles/dev-local.toml \
  --audit /tmp/solum-h3-audit.jsonl \
  --consent-store /tmp/solum-h3-consent.jsonl \
  --token solum-demo-local-token-not-for-production \
  --ehrbase-url http://127.0.0.1:8081/ehrbase
```

## Follow-ups (not this slice)

H3.1–H3.6: FHIR façade + AQL proxy, migration toolkit, subject bridge, partner API docs, Showcase Path E+, MDCG review — see Showcase [`H3-PILOT-CHECKLIST.md`](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H3-PILOT-CHECKLIST.md).
