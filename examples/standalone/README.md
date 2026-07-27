# Standalone reference deployment (Mode A)

This example is **Modus A** from [`docs/INTEGRATION-ROADMAP.md`](../../docs/INTEGRATION-ROADMAP.md):

| Modus | Storage | Auth | Ferrum-Abhängigkeit |
|---|---|---|---|
| **A. Standalone** | Kunde (BYO) | Kunde/SMART-on-FHIR | keine |

It proves that the current Solum CLI (`stage1-baseline-cli-2026-07-26` and later) works as a compliance layer against a **fictional existing EHR/DB** — consent, Crypt4GH field encrypt/decrypt, and audit — with **zero Ferrum imports or APIs**.

Run from the repository root:

```bash
./examples/standalone/run.sh
```

`verify.sh` section 7 invokes the same script.
