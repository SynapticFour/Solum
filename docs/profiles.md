# Jurisdiction profiles

Profiles live in [`config/profiles/`](../config/profiles/). Each file declares:

| Section | Purpose |
|---------|---------|
| `encryption` | Field categories that must be encrypted; allowed key-custody modes |
| `audit` | Mandatory event types; HELIOS export readiness flag |
| `retention` | Default / audit / per-category retention (days) |
| `storage` | Allowed regions + residency enforcement |
| `consent` | Workflow variant + required purposes |
| `regulatory` | Annex / statute references (documentation) |

## Startup validation

```rust
solum_profiles::validate_startup(&profile, &runtime)?;
// Err(ProfileError::StartupRefused { .. }) → abort process
```

Example refusal: profile `eu-ehds` allows only `EU` / `EEA`, runtime sets `storage_region = "us-east-1"`.

## CLI smoke check

```bash
# conforming
cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

# contradictory (must exit non-zero)
SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```
