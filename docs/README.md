# Solum documentation

Every document under `docs/`, grouped as a tutorial, how-to guides, reference, and explanation. One line each.

Solum enforces jurisdiction policy, translates interchange, and records evidence of that processing. The default track sits beside an existing clinical system. An optional second track can front an openEHR store (EHRbase) for partner APIs. That second track is not a hospital record-system interface. See [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) and [H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md).

## Tutorial

| Document | What it is |
|----------|------------|
| [GETTING-STARTED.md](GETTING-STARTED.md) | Rust, libsodium, `make prove`, the profile check, and the first CLI commands. |

## How-to guides

| Document | What it is |
|----------|------------|
| [customer/SIDECAR-INTEGRATION.md](customer/SIDECAR-INTEGRATION.md) | Call the HTTP service from another language. |
| [customer/DEPLOYMENT-RUNBOOK.md](customer/DEPLOYMENT-RUNBOOK.md) | Install and run a pilot on the operator's machine. |
| [customer/DEMO-OPERATOR.md](customer/DEMO-OPERATOR.md) | Run the hospital sign-in demo. Not a clinical pilot. |
| [customer/PARTNER-EHR-API.md](customer/PARTNER-EHR-API.md) | The partner API for a user interface on the optional clinical store. |
| [FOR-MANUFACTURERS.md](FOR-MANUFACTURERS.md) | Which documents a manufacturer can cite in its own technical file, and which are planned. |
| [INCIDENT_RESPONSE.md](INCIDENT_RESPONSE.md) | What an operator does when access, keys, or the log look wrong. |
| [AUTH-HOSPITAL.md](AUTH-HOSPITAL.md) | Clinic sign-in. Not an app launch, and not Ferrum Passports. |
| [AUTH-BROKER.md](AUTH-BROKER.md) | Trust a broker's issuer and key set. No visa decoder. |
| [WORKED-EXAMPLE.md](WORKED-EXAMPLE.md) | Track A commands for consent, field encryption, and the log. |
| [H3-EHRBASE-SPIKE.md](H3-EHRBASE-SPIKE.md) | Bring up EHRbase and the façade for a spike. |
| [H3-EHRBASE-BACKUP.md](H3-EHRBASE-BACKUP.md) | Back up and restore the optional clinical store. |
| [H3-CDR-BACKUP-DRILL.md](H3-CDR-BACKUP-DRILL.md) | A checklist for signing off that backup drill. |
| [H3-WORKED-EVIDENCE.md](H3-WORKED-EVIDENCE.md) | Package the Track B smoke output from Solum-Demo. |
| [MIGRATION-STRANGLER.md](MIGRATION-STRANGLER.md) | Move from a legacy store toward the optional clinical store. |
| [MIGRATION-CUTOVER-CHECKLIST.md](MIGRATION-CUTOVER-CHECKLIST.md) | The checklist for that cut-over. |
| [solum-subject-bridge-runbook.md](solum-subject-bridge-runbook.md) | Operator steps for the subject bridge. |
| [FHIR-VALIDATION.md](FHIR-VALIDATION.md) | Check an IPS-oriented export against a FHIR validator. |
| [CI.md](CI.md) | Which workflows run on a push, and which are started by hand. |

## Reference

| Document | What it is |
|----------|------------|
| [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md) | Security and compliance map for a pilot review. Not a certificate. |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Adversaries and the risk that remains. |
| [FOR-EVALUATORS.md](FOR-EVALUATORS.md) | What this tree tests, and what it does not claim. |
| [BASELINE.md](BASELINE.md) | The Stage 1 snapshot of what is in the build. |
| [profiles.md](profiles.md) | Jurisdiction profile schema, and which files are drafts. |
| [COMPATIBILITY.md](COMPATIBILITY.md) | API and release compatibility, and the licence change date. |
| [LICENSE-OPTIONS.md](LICENSE-OPTIONS.md) | Why BUSL-1.1 is the licence on this repository. |
| [CRYPTO.md](CRYPTO.md) | The Crypt4GH envelope shared with Ferrum, used here on fields. |
| [IDENTITY.md](IDENTITY.md) | Who the layer is for, and which jobs stay in other repositories. |
| [PURPOSE-AND-DUO.md](PURPOSE-AND-DUO.md) | Local purpose strings are not GA4GH DUO visas. |
| [CLAIMS-PROOF-TRAIL.md](CLAIMS-PROOF-TRAIL.md) | Which command supports which statement, and which sentence is forbidden. |
| [DE-FHIR-GAP.md](DE-FHIR-GAP.md) | Gap notes against German FHIR references. Not a telematics claim. |
| [DE-ADAPTER-SPIKE.md](DE-ADAPTER-SPIKE.md) | The narrow clinic-adapter mapping in this repository. A full ISiK guide is still gated. |
| [PRIORITIES.md](PRIORITIES.md) | The living engineering list after the validator pass. |
| [H3-CLINICAL-MODELLING.md](H3-CLINICAL-MODELLING.md) | What the clinical model does and does not claim. |
| [H4-OFFLINE-SYNC-POLICY.md](H4-OFFLINE-SYNC-POLICY.md) | Offline consent, sync, and residency for the Kenya pack. |
| [H5-KEY-CUSTODY-MANAGED.md](H5-KEY-CUSTODY-MANAGED.md) | Key custody for a managed single tenant. |

Files outside `docs/` that the same reading needs: [CHANGELOG.md](../CHANGELOG.md), [RELEASING.md](../RELEASING.md), [VERSIONS](../VERSIONS), [SECURITY.md](../SECURITY.md), [LICENSE](../LICENSE), [LICENSE-COMPATIBILITY.md](../LICENSE-COMPATIBILITY.md).

## Explanation

| Document | What it is |
|----------|------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tracks, crates, startup refusal, and the limit on zero-knowledge claims. |
| [PRODUCT-DEFINITION.md](PRODUCT-DEFINITION.md) | What the product is, and what it refuses to be. |
| [adr/0001-openehr-cdr-and-migration.md](adr/0001-openehr-cdr-and-migration.md) | Decision: optional openEHR store and a strangler migration. |
| [adr/0002-cdr-engine-ehrbase.md](adr/0002-cdr-engine-ehrbase.md) | Decision: EHRbase as the engine for that store. |
| [adr/0003-subject-bridge.md](adr/0003-subject-bridge.md) | Decision: how a clinical subject links to a genomic subject. |
| [adr/0004-internal-network-bind.md](adr/0004-internal-network-bind.md) | Decision: when a non-loopback bind is allowed. |
| [roadmap.md](roadmap.md) | Stage 1 against later stages. |
| [ferrum.md](ferrum.md) | What is reused from Ferrum, and what is only linked. |
| [helios.md](helios.md) | Solum exports a log. HELIOS signs. Signing is not inside Solum. |
| [ECOSYSTEM.md](ECOSYSTEM.md) | Solum beside Ferrum, HELIOS, Lab Kit, and the demo. |
| [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md) | How a Ferrum companion was scoped. Solum still runs without Ferrum. |
| [COMMERCIAL.md](COMMERCIAL.md) | How a commercial licence is requested. The file is not a signed grant. |
| [README.md](README.md) | This index. |

## Do not duplicate Ferrum docs

GA4GH services (Beacon, DRS, htsget, WES, TES, TRS), Passports, and Crypt4GH envelope details for genomic objects are documented upstream:

- [Ferrum README](https://github.com/SynapticFour/Ferrum/blob/main/README.md)
- [Ferrum COMPLIANCE](https://github.com/SynapticFour/Ferrum/blob/main/docs/COMPLIANCE.md)
- [Ferrum Lab Kit](https://github.com/SynapticFour/Ferrum-Lab-Kit)

## Public-repo hygiene

Do not commit pricing, named sales partner shortlists, trademark candidate lists, or internal go-to-market checklists. Keep those outside this tree (see PRODUCT-DEFINITION §10).

*[← Back to the README](../README.md)*
