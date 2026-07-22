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
| `kenya-dpa.toml` | Planned |
| `nigeria-ndpa.toml` | Planned |
| `south-africa-popia.toml` | Planned |

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
