# Jurisdiction profiles

Profiles live in [`config/profiles/`](../config/profiles/). Each file is a jurisdiction (or regime) — **data, not code branches**.

## Schema (per profile)

| Section | Purpose |
|---------|---------|
| `encryption` | Field categories that must be encrypted; allowed key-custody modes |
| `audit` | Mandatory event types; HELIOS export readiness flag |
| `retention` | Default / audit / per-category retention (days) |
| `storage` | Allowed regions + residency enforcement |
| `consent` | Workflow variant + required purposes |
| `regulatory` | Annex / statute references (documentation aids) |

## Present and planned files

| File | Status |
|------|--------|
| `eu-ehds.toml` | Present — EU EHDS Annex II orientation |
| `kenya-dpa.toml` | Present (draft — pending legal review, see regulatory.notes) — Kenya DPA 2019 + Digital Health Act 2023 |
| `nigeria-ndpa.toml` | Planned |
| `south-africa-popia.toml` | Planned |

### Kenya draft

`kenya-dpa.toml` is a **draft** profile (not production-ready). Open legal-review items are recorded in that file’s `regulatory.notes` and in the header `STATUS: DRAFT` comment, including:

- retention periods (7300 days / Digital Health Act s.25 vs DPA s.39 for private deployments)
- audit-log retention (no ODPC-specified figure found)
- `required_purposes` catalogue (guidance-directed, not a statutory list)
- cross-border transfer basis (KE primary residency only; DPA Part VI / DHA s.47 not modelled)
- Digital Health Act serving-copy / national Health Data Bank obligations (outside current schema)

Do not use this profile for a real deployment until those items are closed.

Adding a jurisdiction: copy an existing TOML, adjust fields, drop it in the directory. `load_profiles_dir` picks up every `*.toml` without a code change (unless the schema itself is extended).

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
