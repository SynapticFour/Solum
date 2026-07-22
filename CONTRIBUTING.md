# Contributing to Solum

Solum is licensed under the **Business Source License 1.1** (BUSL-1.1), with parameters and grant text aligned to [Ferrum](https://github.com/SynapticFour/Ferrum) (product name and repo URL adapted); see [LICENSE](LICENSE).

---

## MDCG boundary — answer before you build

> **Mandatory process rule (not optional documentation).**
>
> Before proposing or implementing any new feature, answer:
>
> ### Interpretiert das klinische Daten zur Diagnose-/Therapie-Unterstützung?
>
> | Answer | Action |
> |--------|--------|
> | **Nein** — transport, storage, encryption, audit, consent, residency, schema validation without clinical interpretation | Proceed; record the answer in the PR/issue template |
> | **Ja** — scores, triage, decision support, risk flags, therapy suggestions, automated clinical conclusions | **Stop.** Treat as potential medical-device / IVD / AI Act territory; escalate before coding |
> | **Unsicher** | Do **not** merge speculation. Open an issue/PR with the MDCG question filled as a **required** field and wait for product/regulatory review |

Solum’s default posture is a **compliance layer** (jurisdiction profiles, encryption, audit evidence, FHIR/openEHR interchange) — **not** a medical device. Crossing into diagnosis/therapy support changes the regulatory perimeter of this repository.

Reference: [docs/PRODUCT-DEFINITION.md](docs/PRODUCT-DEFINITION.md) (full product-definition document to be linked when published).

Issue template: [`.github/ISSUE_TEMPLATE/feature.md`](.github/ISSUE_TEMPLATE/feature.md)  
PR template: [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md)

---

## Scope

- **Do** add Solum-specific clinical compliance logic in this repo (`crates/*`).
- **Do not** re-implement GA4GH or copy Ferrum services — depend on git-pinned `ferrum-core` via `solum-crypto` (Lab Kit pattern).
- **Do not** patch Ferrum for Solum-only needs without an explicit product decision from the Ferrum maintainer.
- Prefer documenting Ferrum behaviour with **links**, not duplicated GA4GH guides.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Bump the Ferrum pin by updating **both**:

1. `ferrum-core` `rev` in `crates/crypto/Cargo.toml`
2. `config/ci/ferrum-revision.txt`

## Pull requests

1. One logical change per PR.
2. MDCG question completed (required checklist).
3. Update docs when changing the profile TOML schema.
4. CI must pass (`fmt`, `clippy -D warnings`, tests).
