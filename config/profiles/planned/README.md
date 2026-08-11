# Planned jurisdiction profiles (not auto-loaded)

Files in this directory are **draft scaffolds** for future jurisdictions. They are
**not** picked up by `load_profiles_dir("config/profiles")` (non-recursive).

Do **not** point a pilot `--profile` at these until counsel + engineering review
promote a copy into `config/profiles/*.toml` with an honest STATUS banner.

| Draft | Target regime | Status |
|-------|---------------|--------|
| `nigeria-ndpa.toml` | Nigeria NDPA 2023 | **DRAFT scaffold** — schema-shaped; not counsel-reviewed |
| `south-africa-popia.toml` | South Africa POPIA | **DRAFT scaffold** — schema-shaped; not counsel-reviewed |

Promotion checklist (same for each):

1. Counsel confirmation of retention, transfer destinations, purpose catalogue
2. Move/copy into `config/profiles/` with STATUS header updated
3. Update [docs/profiles.md](../../docs/profiles.md) Present table
4. Add load + refuse tests (wrong region / ephemeral) mirroring Kenya K2
