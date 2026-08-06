# Solum — Deployment Runbook (pilot / on-premise)

**Audience:** IT administrators installing and operating a Stage‑1 Solum pilot
**Companion document:** [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) (security & limitations for IT/legal)
**Authoritative frozen state:** Always read **[docs/BASELINE.md](../BASELINE.md)** for the current verified commit, test posture, and accepted risks. Do not treat numbers or feature lists in older emails as authoritative.

This runbook is derived from the public repository README, profile docs, and baseline. It is **not** a substitute for your organisation’s change-control, backup, or DPIA processes.

---

## 1. Prerequisites

### How Solum is delivered

| Path | Status |
|------|--------|
| **From source** | Supported today — build the `solum` CLI with the Rust toolchain (below). Prefer a **baseline tag** in [BASELINE.md](../BASELINE.md) or a SemVer tag once one exists. |
| **GitHub Release assets** | Prepared — [`.github/workflows/release.yml`](../../.github/workflows/release.yml) builds platform tarballs on `v*` tags. **Use release assets only after a verified SemVer tag exists** (see [RELEASING.md](../../RELEASING.md)). Until then, treat binary install as unavailable and build from source. |

**Stage‑1 evaluation:** Solum is a supervised Stage‑1 evaluation companion — not an unsupervised production EHR. Kenya profile remains **DRAFT**.

### Build prerequisites (from-source)

From the repository README / contributing docs:

| Requirement | Notes |
|-------------|--------|
| Rust **1.91.1** | Via `rust-toolchain.toml` / rustup |
| **libsodium** development library | Required by Crypt4GH (e.g. `brew install libsodium` on macOS, `apt install libsodium-dev` on Linux) |
| Git checkout of Solum | Prefer a **baseline tag** listed in [BASELINE.md](../BASELINE.md) for reproducible pilots |

Example build/run (from README):

```bash
cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

For a durable binary on a host: `cargo build --release -p solum-core` and place `target/release/solum` on a controlled path. After a GitHub Release exists, extract the matching `solum-*.tar.gz` asset instead.

### What `./scripts/verify.sh` is (and is not)

`verify.sh` is a **developer / CI baseline tool** (format, lint, full test suite, license/advisory checks, reference demo deployments). It is **not** a production health check and should **not** be sold or scheduled as the customer’s operational monitoring. See §7.

---

## 2. Choose a jurisdiction profile

Profiles live under `config/profiles/` as TOML **data** — not country-specific code forks. ([profiles.md](../profiles.md); [config/profiles/README.md](../../config/profiles/README.md))

| File | Operator status |
|------|-----------------|
| `eu-ehds.toml` | Present — EU EHDS Annex II–oriented; typical Stage‑1 starting point. **Not** a legal compliance certificate. Allows **customer_held** only (ephemeral refused). |
| `kenya-dpa.toml` | Present **draft** — pending legal review. **Do not use for a real deployment** until open items in that file / [profiles.md](../profiles.md) are closed. Allows **customer_held** only. |
| `dev-local.toml` | **Developer demos only** — permits `ephemeral_test`. Never for paid evaluations or real patient data. |
| `nigeria-ndpa.toml` | Planned |
| `south-africa-popia.toml` | Planned |

You may copy an existing TOML, adjust fields, and drop it into the directory; the loader picks up every `*.toml` without a code change (unless the schema itself is extended). ([profiles.md](../profiles.md))

At startup Solum **refuses to run** if runtime storage region, key-custody posture, mandatory audit events, or consent workflow contradict the active profile. ([profiles.md](../profiles.md); [PRODUCT-DEFINITION.md](../PRODUCT-DEFINITION.md))

---

## 3. First-time bring-up

Use a **dedicated working directory** for audit and consent stores (paths below are examples from README).

```bash
PROFILE=config/profiles/eu-ehds.toml
AUDIT=/var/lib/solum/audit.jsonl          # choose your paths
CONSENT=/var/lib/solum/consent.jsonl
mkdir -p /var/lib/solum

# 1. Profile / runtime conformance
cargo run -p solum-core -- check --profile "$PROFILE"

# Expect failure when residency is wrong (example):
SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile "$PROFILE"
# → non-zero exit
```

### Consent (grant / status / revoke)

Mutating consent commands require `--capability` (GTM-1). Omit it → fail-closed denial. `--scope` on grant is still a **consent data category**, not an authorization capability.

```bash
cargo run -p solum-core -- consent grant \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor practitioner/7 \
  --capability solum:consent:grant \
  --scope patient_summary

cargo run -p solum-core -- consent status \
  --profile "$PROFILE" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision
# → granted | revoked | unknown  (read-only; no --audit required)

cargo run -p solum-core -- consent revoke \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor patient/42 \
  --capability solum:consent:revoke
```

### Field encrypt / decrypt (CustomerHeld — evaluation / pilot path)

```bash
mkdir -p /tmp/solum-demo
echo 'demo-plaintext' > /tmp/solum-demo/plain.txt

# Operator-controlled keypair file (not HSM; protect with filesystem permissions)
cargo run -p solum-core -- crypto keygen \
  --key-ref customer/eval-1 --out /tmp/solum-demo/customer.keypair.json

cargo run -p solum-core -- crypto encrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --category patient_summary --key-ref customer/eval-1 \
  --keypair /tmp/solum-demo/customer.keypair.json \
  --actor practitioner/7 --capability solum:crypto:encrypt \
  --in /tmp/solum-demo/plain.txt --out /tmp/solum-demo/field.json

cargo run -p solum-core -- crypto decrypt \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --key-ref customer/eval-1 \
  --keypair /tmp/solum-demo/customer.keypair.json \
  --actor practitioner/7 --capability solum:crypto:decrypt \
  --in /tmp/solum-demo/field.json --out /tmp/solum-demo/plain-out.txt
```

### Audit export / verify

```bash
cargo run -p solum-core -- audit export --audit "$AUDIT" --out /var/lib/solum/helios.json
cargo run -p solum-core -- audit verify --audit "$AUDIT"
# → ok
```

Protect filesystem permissions on `$AUDIT`, `$CONSENT`, and any key material according to your organisation’s standards. Solum does not replace OS-level access control.

---

## 4. Key management

### Paid evaluation / pilot path (required)

**Paid evaluations and pilots must use CustomerHeld `--keypair` files** (or the library / AWS KMS APIs). Do **not** use `--ephemeral` or describe ephemeral keys as an evaluation or production custody option.

| Mechanism | How | Notes |
|-----------|-----|--------|
| CLI CustomerHeld file | `crypto keygen` → `--keypair` on encrypt/decrypt | Operator-controlled JSON (pubkey + privkey). Not an HSM. Protect with OS permissions (0600 on Unix). |
| Library `CustomerHeldKeyProvider` | Register keypairs generated outside Solum | Same custody posture as CLI `--keypair`. |
| AWS KMS envelope (optional feature) | Library `wrap_seed` / `from_wrapped_seed` | Feature default off; **not** a CLI command. See [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) §4. |

Pilot profiles (`eu-ehds`, `kenya-dpa`) list only `customer_held`. Declaring `EphemeralTest` runtime custody (CLI `--ephemeral`, or `SOLUM_KEY_CUSTODY=ephemeral_test` on `check`) **refuses startup**.

### Dev-only ephemeral path (forbidden for paid evaluation)

`--ephemeral` requires **both**:

1. `SOLUM_ALLOW_EPHEMERAL=1` (or `true` / `yes`), and
2. A profile that allows `ephemeral_test` (e.g. `config/profiles/dev-local.toml`).

It writes a plaintext `*.ephemeral-keypair.json` sidecar (0600 on Unix). **Never for real patient data or paid evaluations.**

Neither customer-held nor AWS-KMS providers claim TEE isolation; private seeds use best-effort `ZeroizeOnDrop` in process memory. ([BASELINE.md](../BASELINE.md))

---

## 5. Consent and authorization setup

### Consent purposes

Only purposes listed in the active profile’s required-purpose catalogue are accepted. Unknown purposes are rejected **before** consent or audit writes (same posture as unknown encryption categories). ([BASELINE.md](../BASELINE.md); product orchestration behaviour)

### Capabilities (GTM‑1)

When your integration uses the **structured actor** API (actor identity with scopes), assign **exact** capability strings. Fail-closed: missing or empty scopes → deny + audit event; no side effects.

The **CLI** builds that structured actor from `--actor` plus one or more `--capability` flags and always calls the checked APIs. Omit `--capability` → empty scopes → denial (same fail-closed rule).

Documented capability strings in this baseline:

| Capability | Operation gated |
|------------|-----------------|
| `solum:consent:grant` | Grant consent |
| `solum:consent:revoke` | Revoke consent |
| `solum:crypto:encrypt` | Encrypt a clinical field category |
| `solum:crypto:decrypt` | Decrypt a clinical field |

Encrypt does **not** imply decrypt. There is no wildcard hierarchy (e.g. no `solum:*`). ([BASELINE.md](../BASELINE.md))

### Warning — legacy library plain-string actors

Library callers that still invoke grant/revoke/encrypt/decrypt with a plain actor string (no scopes) **bypass** GTM‑1 authorization. The shipped CLI no longer uses that path. Treat remaining library legacy callers as a known open flank until they migrate. ([BASELINE.md](../BASELINE.md); [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md) §5 and §8)

**ANNAHME, bitte prüfen:** How your IdP / SMART-on-FHIR or Ferrum Passport scopes are mapped into these exact capability strings is an integration design choice for your project — the baseline freezes the check mechanism, not a full hospital role catalogue.

---

## 6. Audit log operations

| Task | CLI (from README) | Notes |
|------|-------------------|--------|
| Export HELIOS-oriented JSON | `solum … audit export --audit … --out …` | Export envelope only; live HELIOS signing is **deferred / not productized** ([helios.md](../helios.md)) |
| Verify hash chain | `solum … audit verify --audit …` | Detects tampering / broken chain for the file store |
| Retention | Per profile `retention` section | Operator responsibility to enforce archival / deletion outside Solum if required |

**Single-writer assumption:** Stage 1’s durable file audit store is designed for **one writer**. Do not run multiple Solum instances appending to the same audit file concurrently — multi-writer backends are out of this baseline. ([BASELINE.md](../BASELINE.md))

Back up audit and consent store files with the same diligence as other regulated logs. Solum’s hash chain detects post-write tampering of the log file; it does not replace offline backups.

---

## 7. Monitoring and health checks

### Developer / release verification (not production monitoring)

`./scripts/verify.sh` validates that a **source tree** matches Synaptic Four’s Stage‑1 quality bar (toolchain, formatting, tests, advisory policy, demo reference deployments). Run it when rebuilding from a tagged baseline — **not** as a cron “is the clinic OK?” probe.

### What operators should check in production (suggested)

These checks are **operational practice** derived from product behaviour; they are not a separate monitoring product:

1. **Startup / profile check** succeeds under the intended storage region and key-custody posture (`solum check` or equivalent process start).
2. **Audit chain verify** succeeds on a schedule against the live audit file.
3. **Filesystem health** for audit/consent paths (permissions, disk space, backup success).
4. **Authorization denials** appearing in audit export when unexpected (may indicate mis-scoped actors or probing).
5. **Key material handling** — confirm you are **not** using ephemeral CLI sidecars for real data; confirm KMS/customer-held integration matches your DPIA.

**ANNAHME, bitte prüfen:** Alerting thresholds, SIEM ingestion of HELIOS JSON, and on-call runbooks are site-specific and not defined in this repository.

---

## 8. Known limitations

Do not duplicate the full risk register here. Read **[SECURITY-OVERVIEW.md §8](SECURITY-OVERVIEW.md#8-known-limitations-do-not-skip)** and the authoritative lists in **[BASELINE.md](../BASELINE.md)** (*Bewusst akzeptierte Risiken* / *Explizit außerhalb dieser Baseline*).

High-signal operational reminders:

- Paid evaluation / pilot crypto = CustomerHeld `--keypair` (not `--ephemeral`)
- Library plain-string actors = no capability enforcement (CLI uses `--capability`, fail-closed if omitted)
- Kenya profile = draft only
- GitHub Release binaries available only after a verified `v*` tag (otherwise from-source)
- Audit store = single writer; HELIOS live signing not productized
- AWS KMS provisioning = library only, optional feature

---

## 9. Where to get help

- [contact@synapticfour.com](mailto:contact@synapticfour.com) · [synapticfour.com](https://synapticfour.com)
- Cite your **baseline tag** and [BASELINE.md](../BASELINE.md) commit when opening a support or security discussion.
- Security posture narrative for IT/legal: [SECURITY-OVERVIEW.md](SECURITY-OVERVIEW.md)
