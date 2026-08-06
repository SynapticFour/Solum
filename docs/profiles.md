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
| `kenya-dpa.toml` | Present (**PROVISIONAL-PRODUCTION-CANDIDATE** — non-counsel Vorprüfung applied; real counsel still required) — Kenya DPA 2019 + Digital Health Act 2023 (`customer_held` only) |
| `dev-local.toml` | Developer demos only — allows `ephemeral_test`; never for paid evaluations |
| `nigeria-ndpa.toml` | Planned |
| `south-africa-popia.toml` | Planned |

### Kenya (provisional)

`kenya-dpa.toml` is a **PROVISIONAL-PRODUCTION-CANDIDATE** after a **non-counsel** Vorprüfung ([KENYA-K1-VORPRUEFUNG.md](counsel/KENYA-K1-VORPRUEFUNG.md)). It is **not** PRODUCTION, **not** ODPC-certified, and **not** for live patient system-of-record until qualified Kenya counsel confirms [KENYA-K1-BRIEF.md](counsel/KENYA-K1-BRIEF.md).

Engineering posture after Vorprüfung:

- Retention `7300` days = conservative Digital Health Act–aligned **default**, not “Kenya requires 20 years for all private deployments”
- Audit `7300` = evidence retention (no ODPC figure claimed)
- `required_purposes` = primary-care floor; research etc. in `optional_purposes` only
- `permitted_destinations = []` fail-closed (strength); mechanisms are pathways, not permits
- National Health Data Bank = operator obligation / Solum non-goal
- Edge offline policies documented in `regulatory.notes` (enforcement = later K2/K3)

**Portfolio decision:** Kenya is the **first non-EU geography** to drive toward production-ready (provisional). Work breakdown: [H4 geography decision](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H4-GEOGRAPHY-DECISION.md) (K1 legal / K2 technical / K3 field).

Adding a jurisdiction: copy an existing TOML, adjust fields, drop it in the directory. `load_profiles_dir` picks up every `*.toml` without a code change (unless the schema itself is extended).

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
cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
# expect non-zero exit
```
