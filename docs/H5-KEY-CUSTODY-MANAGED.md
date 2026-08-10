# H5 — Key custody for managed single-tenant

**Audience:** operators offering Synaptic Four as **managed single-tenant** (hosted on-prem)
**Horizon:** optional H5 preparedness — **not** multi-tenant SaaS
**Portfolio:** Showcase [ADR 0003](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/adr/0003-tenant-boundaries.md) · [H5-MANAGED-SINGLE-TENANT.md](https://github.com/SynapticFour/SynapticFour-Showcase/blob/main/docs/pilots/H5-MANAGED-SINGLE-TENANT.md)

## Preference order (managed deploy)

| Rank | Model | When |
|------|--------|------|
| 1 | **CustomerHeld** file / HSM under customer control | Default — same as on-prem pilot |
| 2 | Customer-controlled cloud CMK (e.g. Solum optional `aws-kms` envelope) | Customer owns the KMS key policy; Synaptic Four operates the VM |
| 3 | Operator-held keys | Only with written customer acceptance; avoid for clinical SoR |

Ephemeral keys remain **dev-only** (`dev-local` + `SOLUM_ALLOW_EPHEMERAL`) — never for managed patient data.

## Honesty

- Envelope unwrap (optional AWS KMS) lands seeds in **process memory** with best-effort `ZeroizeOnDrop` — **not** HSM/TEE/FIPS certification.
- One managed deployment = one tenant; never share Crypt4GH key material or sidecar tokens across customers.

## TEE / “honest ZK” sketch (not implemented)

Solum’s documented long-term path (see [architecture.md](architecture.md)):

1. Customer-held keys at rest
2. Confidential computing / TEE where plaintext must be touched
3. Complete, customer-inspectable auditability

**H5 does not implement TEE.** Do not market managed hosting as confidential computing. Revisit only under an explicit security programme.

Optional audit correlation: set `SOLUM_TENANT_ID` so evidence packs show which managed tenant produced the trail (metadata only).
