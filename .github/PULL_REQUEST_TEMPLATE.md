## Summary

<!-- What does this PR change and why? -->

## MDCG boundary (required)

**Interpretiert das klinische Daten zur Diagnose-/Therapie-Unterstützung?**

- [ ] **Nein** — no diagnosis/therapy interpretation (compliance, transport, storage, audit, consent, schema, residency, encryption only)
- [ ] **Ja** — clinical interpretation / decision support (must not proceed without regulatory review)
- [ ] **Unsicher** — needs product/regulatory decision before merge

Justification (required; one short paragraph):

<!-- Explain why the selected answer applies. Link PRODUCT-DEFINITION.md / issue if review is needed. -->

## Profile / startup impact

- [ ] No change to jurisdiction profile schema or startup validation
- [ ] Profile schema or `validate_startup` changed — docs + tests updated
- [ ] Verified contradictory config still refuses start (if validation touched)

## Test plan

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Manual: `cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml`
