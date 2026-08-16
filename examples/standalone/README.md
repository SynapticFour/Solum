# Standalone reference deployment (Mode A)

This example is **Modus A** from [`docs/INTEGRATION-ROADMAP.md`](../../docs/INTEGRATION-ROADMAP.md):

| Modus | Storage | Auth | Ferrum-Abhängigkeit |
|---|---|---|---|
| **A. Standalone** | Kunde (BYO) | Kunde/SMART-on-FHIR | keine |

It proves that the current Solum CLI works as a compliance layer against a **fictional existing EHR/DB** — consent, Crypt4GH field encrypt/decrypt via CustomerHeld `--keypair`, and audit — with **zero Ferrum imports or APIs**. Live hospital identity is clinic OIDC or SMART **Backend Services** (not SMART App Launch); see [AUTH-HOSPITAL.md](../../docs/AUTH-HOSPITAL.md).

Run from the repository root:

```bash
./examples/standalone/run.sh
```

`verify.sh` section 7 invokes the same script.
