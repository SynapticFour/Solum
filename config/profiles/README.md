# Jurisdiction profiles

Each `*.toml` file is a jurisdiction profile loaded by `solum-profiles`.

| File | Status |
|------|--------|
| `eu-ehds.toml` | Present — EU EHDS (Annex II–oriented) |
| `kenya-dpa.toml` | Planned — Kenya data-protection / digital health framework |
| `nigeria-ndpa.toml` | Planned — Nigeria NDPA-oriented |
| `south-africa-popia.toml` | Planned — POPIA-oriented |

## Extending without code changes

1. Copy `eu-ehds.toml` as a starting point.
2. Set `meta.profile` / `meta.jurisdiction` and adjust encryption, audit, retention, storage, and consent sections.
3. Place the file here. `load_profiles_dir` picks up every `*.toml`.

Schema version is `schema_version = 1` (see `solum_profiles::PROFILE_SCHEMA_VERSION`).

See also [docs/profiles.md](../../docs/profiles.md) and [docs/PRODUCT-DEFINITION.md](../../docs/PRODUCT-DEFINITION.md) §6.
