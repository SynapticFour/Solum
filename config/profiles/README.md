# Jurisdiction profiles

Each `*.toml` file is a jurisdiction profile loaded by `solum-profiles`.

| File | Status |
|------|--------|
| `eu-ehds.toml` | Initial — EU EHDS (Annex II-oriented) |
| `ke-*.toml` | Planned — Kenya |
| `ng-*.toml` | Planned — Nigeria |
| `za-*.toml` | Planned — South Africa |

## Extending without code changes

1. Copy `eu-ehds.toml` as a starting point.
2. Set `meta.profile` / `meta.jurisdiction` and adjust encryption, audit, retention, storage, and consent sections.
3. Place the file in this directory. `load_profiles_dir` picks up every `*.toml`.

Schema version is `schema_version = 1` (see `solum_profiles::PROFILE_SCHEMA_VERSION`).
