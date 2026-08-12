# Relationship to HELIOS

[HELIOS](https://github.com/SynapticFour/HELIOS) is a separate Synaptic Four evidence tool (signed, reproducible attestations). It is **not** vendored into this workspace.

## Status: export + external HELIOS ingest (productized)

Solum prepares a stable audit export. **Operators run HELIOS separately** to validate clinical access evidence and optionally sign a report. Solum does **not** embed HELIOS signing keys or call HELIOS as an in-process step.

### What exists

- `solum-audit` event model and durable hash-chained file store
- Stable JSON export: **`solum-audit-helios-chain-v1`** via sidecar `GET /v1/audit/export` or `solum-core audit export`
- HELIOS check **`CLIN-ACCESS-001`** ingesting that export
- Documented recipe: [HELIOS `docs/solum-ingest.md`](https://github.com/SynapticFour/HELIOS/blob/main/docs/solum-ingest.md) · `helios solum-audit --export …` · `make solum-clinical-evidence`

### What still does **not** exist

- Calling HELIOS as a live signing step *inside* the Solum binary
- Productized custody of HELIOS signing keys via Solum
- Automatic attestation submission to third-party auditors

**Portfolio boundary:** HELIOS is a sibling project ([ECOSYSTEM.md](ECOSYSTEM.md)). Showcase golden-path with Solum can run the external ingest after Stage-1 proofs.

## Why Solum still mentions HELIOS

HELIOS is strongest around pipeline / reproducibility-style evidence. Solum needs a related evidence class for **clinical compliance**:

- access-log / authorization evidence
- consent change attestation
- processing-environment attestation (where exported)

Solum **exports**; HELIOS **packages / signs**. Keep custody of signing keys on the HELIOS operator side.

## Boundary

- Solum owns clinical compliance policy and event semantics.
- HELIOS owns evidence packaging / signing mechanics (upstream).
- New HELIOS evidence types are portfolio/upstream requests; implement adapters here, not a fork of HELIOS.
