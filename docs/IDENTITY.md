# Who Solum is for

Solum is a **clinical compliance layer**: consent, audit chain, FHIR/openEHR interchange, customer-held Crypt4GH for clinical fields.

It is a complete product without Ferrum. The Ferrum git pin is for shared crypto/types and an optional companion example — not a runtime requirement to enforce consent on a hospital system.

## Audience

Hospital IT, HMIS integrators, data-protection officers who need fail-closed purpose/capability checks and a tamper-evident audit log.

**Not for:** running Beacon/WES (that is Ferrum), issuing GA4GH Passports (that is ga4gh-infra), a researcher workbench (that is BRA).

## Standalone

```bash
git clone https://github.com/SynapticFour/Solum.git && cd Solum
make prove
```

Interactive proof: [Solum-Demo](https://github.com/SynapticFour/Solum-Demo) `make up && make smoke-ci`.

## Optional composition

| Join | What you gain | Contract |
|------|----------------|----------|
| Ferrum | Same subject id on genomic DRS objects | Ferrum `solum_consent` HTTP; pin ferrum-core in `config/ci/ferrum-revision.txt` |
| BRA | Phenopacket → clinical subject | `POST /v1/cdr/subject-link` with actor, capability, purpose |
| HELIOS | Signed clinical evidence | `solum-audit-helios-chain-v1` export file — HELIOS does not call this sidecar |

See [ECOSYSTEM.md](ECOSYSTEM.md).
