# CI triggers

Push to `main` and pull requests run only the secret scan and dependency review. Product CI is manual. A `v*` tag publishes the GitHub Release binaries and the GHCR sidecar image.

Run `make verify-release` before tagging. That command is `scripts/verify.sh`: fmt, clippy, tests, `cargo deny` when `cargo-deny` is installed, the profile refusal checks, and the feature-path tests when Rust 1.94.1 is installed.

Minutes below are rough runner time, not a measured bill.

| Workflow | Trigger | Rough minutes | When to run manually |
|---|---|---|---|
| `ci.yml` | `workflow_dispatch` | 10–25 | Before a tag, after `make verify-release`, when you want the same jobs on a GitHub runner (including vendored crypt4gh and the aws-kms feature path). |
| `codeql.yml` | `workflow_dispatch` | 10–30 | Before a release when you want a CodeQL pass. No weekly schedule. |
| `release.yml` | tag `v*`; `workflow_dispatch` | 15–30 | Dispatch with `create_release=false` for a dry-run before the first tag. The tag publishes the release. |
| `docker-release.yml` | tag `v*`; `workflow_dispatch` | 10–15 | The tag publishes the sidecar image. Dispatch to rebuild without a new tag. |
| `secret-scan.yml` | pull request; push to `main` or `master` | 1–2 | Leave it. It stays on those events. |
| `dependency-review.yml` | pull request | 1–2 | Leave it. It stays on pull requests. |
