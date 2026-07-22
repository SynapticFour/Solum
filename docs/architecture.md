# Architecture (scaffold)

```
                    ┌─────────────────────────────────────┐
                    │            solum-core               │
                    │  startup validation · orchestration │
                    └───────────────┬─────────────────────┘
           ┌────────────┬───────────┼───────────┬────────────┐
           ▼            ▼           ▼           ▼            ▼
     solum-profiles  solum-crypto  solum-fhir  solum-openehr  solum-audit
     (TOML juris-    (ferrum-core  (stage 1)   (stage 2       (HELIOS export
      diction         git pin)                  scaffold)      prepared)
      profiles)
```

## Startup rule

`solum-profiles::validate_startup` compares the active jurisdiction profile with the runtime configuration (storage region, key custody, mandatory audit events, consent workflow). On contradiction the process **exits with failure** — logging alone is not sufficient.

## Crates

| Crate | Role |
|-------|------|
| `solum-core` | Product orchestration + `solum` CLI (`check`) |
| `solum-profiles` | Load/validate jurisdiction TOML profiles |
| `solum-crypto` | Customer-held keys; pins `ferrum-core` |
| `solum-fhir` | FHIR adapter (stage 1 focus) |
| `solum-openehr` | openEHR adapter (stage 2 scaffold) |
| `solum-audit` | Audit events + HELIOS-oriented JSON export |
