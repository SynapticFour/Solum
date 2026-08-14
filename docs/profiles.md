# Jurisdiction profiles

Profiles live in [`config/profiles/`](../config/profiles/). Each file is a jurisdiction (or regime) — **data, not code branches**.

## Schema (per profile)

| Section | Purpose |
|---------|---------|
| `encryption` | Field categories that must be encrypted; allowed key-custody modes |
| `audit` | Mandatory event types; HELIOS export readiness flag |
| `retention` | Default / audit / per-category retention (days) |
| `storage` | Allowed regions + residency enforcement |
| `consent` | Workflow + `required_purposes` (primary floor) + `optional_purposes` (secondary / research; separate lawful basis) |
| `transfer` | Permitted transfer mechanisms + destinations + serving-copy flag |
| `regulatory` | Annex / statute references (documentation aids) |

## Present and planned files

| File | Status |
|------|--------|
| `eu-ehds.toml` | Present — EU EHDS Annex II orientation (`customer_held` only) |
| `kenya-dpa.toml` | Present (**EVALUATION-ONLY** — non-counsel Vorprüfung applied; real counsel still required; **not** a production candidate) — Kenya DPA 2019 + Digital Health Act 2023 (`customer_held` only) |
| `dev-local.toml` | Developer demos only — allows `ephemeral_test`; never for paid evaluations |
| `planned/nigeria-ndpa.toml` | **DRAFT scaffold** — not auto-loaded; not counsel-reviewed ([planned/README.md](../config/profiles/planned/README.md)) |
| `planned/south-africa-popia.toml` | **DRAFT scaffold** — not auto-loaded; not counsel-reviewed |

### Kenya (provisional)

`kenya-dpa.toml` is **EVALUATION-ONLY** after a **non-counsel** Vorprüfung. It is **not** a production candidate, **not** ODPC-certified, and **not** for live patient system-of-record until qualified Kenya counsel confirms.

Engineering posture after Vorprüfung:

- Retention `7300` days = conservative Digital Health Act–aligned **default**, not “Kenya requires 20 years for all private deployments”
- Audit `7300` = evidence retention (no ODPC figure claimed)
- `required_purposes` = primary-care floor; research etc. in `optional_purposes` only
- `permitted_destinations = []` fail-closed (strength); mechanisms are pathways, not permits
- National Health Data Bank = operator obligation / Solum non-goal
- Edge offline policies: written in [H4-OFFLINE-SYNC-POLICY.md](H4-OFFLINE-SYNC-POLICY.md); field reconcile remains K3

**Portfolio decision:** Kenya is the **first non-EU geography** under evaluation. It is **not** production-ready until counsel. Work breakdown: [H4 geography decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md) · Showcase [H4-PILOT-CHECKLIST.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-PILOT-CHECKLIST.md) (K1 legal / K2 technical / K3 field).

Adding a jurisdiction: copy an existing TOML, adjust fields, drop it into the directory. `load_profiles_dir` picks up every `*.toml` without a code change (unless the schema itself is extended).

## Transfer policy

`[transfer]` is additive beside `[storage]`:

| Field | Meaning |
|-------|---------|
| `permitted_mechanisms` | `safeguards_based`, `hdab_mediated`, and/or `statutory_exception` |
| `permitted_destinations` | Destination labels (`EU`, `EEA`, `KE`, …); empty = not enumerable → every concrete check fails |
| `requires_serving_copy` | Declarative flag (e.g. Kenya strategic-interest serving copy) |

Missing `[transfer]` defaults to **no** mechanisms and **no** destinations (restrictive-by-default). Use `validate_transfer` for a concrete runtime request — it is **not** part of `validate_startup`.

EU-internal EHDS primary-use exchange (MyHealth@EU) is residency / data-space traffic under `[storage]`, not a third-country `TransferMechanism`. EHDS secondary use via HDABs is `hdab_mediated`.

## Startup validation

```rust
solum_profiles::validate_startup(&profile, &runtime)?;
// Err(ProfileError::StartupRefused { .. }) → abort process
```

Example refusal: profile `eu-ehds` allows only `EU` / `EEA`, runtime sets `storage_region = "us-east-1"`.

## CLI smoke check

```bash
# EU — operator must attest residency (not inferred from the profile default)
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
# expect non-zero exit

# Kenya (H4) — storage region must be KE; CustomerHeld only (default check posture)
SOLUM_STORAGE_REGION=KE cargo run -p solum-core -- check --profile config/profiles/kenya-dpa.toml

SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/kenya-dpa.toml
# expect non-zero exit (residency)

SOLUM_KEY_CUSTODY=ephemeral_test SOLUM_STORAGE_REGION=KE \
  cargo run -p solum-core -- check --profile config/profiles/kenya-dpa.toml
# expect non-zero exit (ephemeral refused)

# Sidecar: same region env — CustomerHeld keys required
SOLUM_STORAGE_REGION=KE solum-sidecar \
  --profile config/profiles/kenya-dpa.toml \
  --keys-dir /var/lib/solum/keys \
  --audit /var/lib/solum/audit.jsonl \
  --consent-store /var/lib/solum/consent.jsonl \
  --token "$SOLUM_SIDECAR_TOKEN"
```

`kenya-dpa` remains **EVALUATION-ONLY** until counsel confirms — check success ≠ ODPC clearance.
