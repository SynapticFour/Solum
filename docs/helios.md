# Relationship to HELIOS

[HELIOS](https://github.com/SynapticFour/HELIOS) is a separate Synaptic Four evidence tool (signed, reproducible attestations). It is **not** vendored into this workspace.

## Why Solum talks to HELIOS

HELIOS today is strongest around pipeline / reproducibility-style evidence. Solum needs a related evidence class for **clinical compliance**:

- access-log signing
- consent change attestation
- processing-environment attestation

Where that fits, Solum should **consume** HELIOS (or an equivalent) rather than grow a second cryptographic evidence stack.

## What lives in this repo

`solum-audit` defines an in-repo event model and a stable JSON export envelope (`solum-audit-helios-v1`) so an external evidence tool can ingest Solum trails. Wiring a live HELIOS CLI/API integration is stage work, not a scaffold requirement.

## Boundary

- Solum owns clinical compliance policy and event semantics.
- HELIOS owns evidence packaging / signing mechanics (upstream).
- New HELIOS evidence types are portfolio/upstream requests; implement adapters here, not a fork of HELIOS.
