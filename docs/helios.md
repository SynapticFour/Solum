# Relationship to HELIOS

[HELIOS](https://github.com/SynapticFour/HELIOS) is a separate Synaptic Four evidence tool (signed, reproducible attestations). It is **not** vendored into this workspace.

## Status: deferred / not productized

**Live HELIOS CLI/API signing is not a Solum product feature today.** Do not claim live signing, attestation submission, or a turnkey HELIOS bridge in Stage‑1 evaluations, sales materials, or pilot SOWs.

What exists:

- `solum-audit` event model and durable hash-chained file store
- Stable JSON export envelopes (`solum-audit-helios-v1` / chain variant) so an *external* evidence tool could ingest trails later

What does **not** exist:

- Calling HELIOS as a live signing step inside Solum
- Productized custody of HELIOS signing keys via Solum
- Guaranteed round-trip with a particular HELIOS release

Wiring a live HELIOS integration remains **roadmap / stage work**, not Stage‑1 delivery. See [roadmap.md](roadmap.md).

**Portfolio boundary:** HELIOS is a sibling Synaptic Four project ([ECOSYSTEM.md](ECOSYSTEM.md)). Mentioning HELIOS in Solum docs means “export shape prepared for an external evidence tool,” not “Solum ships HELIOS” or “attestations are signed today.”

## Why Solum still mentions HELIOS

HELIOS today is strongest around pipeline / reproducibility-style evidence. Solum needs a related evidence class for **clinical compliance**:

- access-log signing
- consent change attestation
- processing-environment attestation

Where that eventually fits, Solum should **consume** HELIOS (or an equivalent) rather than grow a second cryptographic evidence stack. Until both ends are released and custody is clear, keep the relationship as an export-shape boundary only.

## Boundary

- Solum owns clinical compliance policy and event semantics.
- HELIOS owns evidence packaging / signing mechanics (upstream).
- New HELIOS evidence types are portfolio/upstream requests; implement adapters here, not a fork of HELIOS.
