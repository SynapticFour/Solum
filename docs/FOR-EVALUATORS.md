# For evaluators

Factual snapshot of this repository. Not a sales brief. Not legal advice. Not a medical-device claim.

## Maturity

**Early access** — Stage-1 / evaluation. First public commit 2026-07-22. Treat this as evaluation software, not a production hospital system.

Shipping core: `config/profiles/eu-ehds.toml`. Kenya (`kenya-dpa.toml`) is **evaluation-only** (not counsel-reviewed, not a production candidate). Nigeria / South Africa under `config/profiles/planned/` are draft scaffolds and are not auto-loaded. Egypt has no profile file in this tree.

Solum does **not** interpret clinical data for diagnosis or therapy ([PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md), [CONTRIBUTING.md](../CONTRIBUTING.md)).

## License

Business Source License 1.1. Additional Use Grant for non-commercial research and internal research use; Change License Apache-2.0 after four years. See [LICENSE](../LICENSE).

## Tested in this tree

| Claim | Evidence |
|-------|----------|
| Workspace tests | `make prove` |
| Profile refuse-closed | `solum check` fails when `SOLUM_STORAGE_REGION` contradicts the profile |
| Interactive Stage-1 smoke | [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) against a **pinned** Solum tag |

`ferrum-core` is git-pinned (crypto/types). Solum is not a Ferrum add-on and not a combo SKU.

## Not tested / not claimed

| Topic | Status |
|-------|--------|
| Production hospital EHR | Not this product. Track B is an optional EHRbase façade. |
| Live HELIOS signing | Audit export is an envelope only. HELIOS ingests the JSON file. |
| Kenya as production | Evaluation profile only. |
| Medical device / MDR | Design intent is not a device; that is not a legal certification. |
| Third-party audit / GA4GH certification | Neither claimed. |

## Contact

Questions can be sent to [contact@synapticfour.com](mailto:contact@synapticfour.com).
