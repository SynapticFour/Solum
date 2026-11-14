# For manufacturers

A manufacturer of a practice, clinic, or health-information system can point its own technical file at the public documents below. This page says what is in the repository, and what is only planned. It is not a certificate and not legal advice. Registry: SOLUM-SUPPLIER-DOCS-001.

## Present

| For the technical file | Where it is |
|------------------------|-------------|
| Changelog | [CHANGELOG.md](../CHANGELOG.md). Version headings follow the dates in that file. |
| Versions list | [VERSIONS](../VERSIONS) names the EHRbase image pins used by the optional clinical store. Product release tags are on the [releases page](https://github.com/SynapticFour/Solum/releases). This page does not equate a tag with the default branch. |
| Integration guide | [customer/SIDECAR-INTEGRATION.md](customer/SIDECAR-INTEGRATION.md). How to call the HTTP service from the manufacturer's own software. |
| Security overview | [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md). [SECURITY.md](../SECURITY.md) is the address for a vulnerability report. |
| Threat model | [THREAT_MODEL.md](THREAT_MODEL.md). |
| Incident response | [INCIDENT_RESPONSE.md](INCIDENT_RESPONSE.md). |
| Public issue list | [GitHub issues](https://github.com/SynapticFour/Solum/issues). |

The getting-started guide, the integration guide, and the [deployment runbook](customer/DEPLOYMENT-RUNBOOK.md) describe how to set Solum up. They do not state support terms. Registry: SOLUM-HANDOVER-001.

## Planned

A software bill of materials is planned. [`.github/workflows/release.yml`](../.github/workflows/release.yml) has an `sbom` job that installs `cargo-cyclonedx` 0.5.9 in the job and runs `cargo cyclonedx --format json`. That program is not a dependency of the Solum binaries. The job is written to attach `solum-core.cdx.json` and `solum-sidecar.cdx.json` to a GitHub Release for a `v*` tag.

The job stored on tag `v0.1.1` failed: that copy passed `--workspace`, which cargo-cyclonedx 0.5.9 rejects. The workflow file on this branch does not pass `--workspace`. A tag runs the workflow stored on the tag, so this copy has not run. The steps are in [RELEASING.md](../RELEASING.md).

GitHub Release `v0.1.1` does have `solum-core.cdx.json` and `solum-sidecar.cdx.json` attached, by the manual steps in that releasing note, after the job failed. Those two files are not in this git tree. Until a tag runs the workflow file that omits `--workspace`, the bill of materials from that job stays planned.
