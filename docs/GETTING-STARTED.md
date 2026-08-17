# Getting started

**Prerequisites:** Rust 1.91.1 (`rust-toolchain.toml`) and **libsodium** (e.g. `brew install libsodium` on macOS, `apt install libsodium-dev` on Linux). `ferrum-core` is git-pinned (`crates/crypto` + `config/ci/ferrum-revision.txt`). For a local Ferrum sibling checkout, see `.cargo/config.toml.example`.

## Zero-risk proof

```bash
make prove
SOLUM_STORAGE_REGION=EU cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml

# Must fail (non-zero): profile requires EU/EEA residency
SOLUM_STORAGE_REGION=us-east-1 cargo run -p solum-core -- check --profile config/profiles/eu-ehds.toml
```

## CLI (evaluation / Stage-1)

The `solum` binary (crate `solum-core`) exposes jurisdiction check plus consent, crypto, and audit subcommands. Paths below are examples.

```bash
PROFILE=config/profiles/eu-ehds.toml
AUDIT=/tmp/solum-demo/audit.jsonl
CONSENT=/tmp/solum-demo/consent.jsonl
mkdir -p /tmp/solum-demo
export SOLUM_STORAGE_REGION=EU

cargo run -p solum-core -- check --profile "$PROFILE"

# Mutating consent requires --capability (omit → denied).
cargo run -p solum-core -- consent grant \
  --profile "$PROFILE" --audit "$AUDIT" --consent-store "$CONSENT" \
  --subject patient/42 --purpose care_provision --actor practitioner/7 \
  --capability solum:consent:grant \
  --scope patient_summary

cargo run -p solum-core -- crypto keygen \
  --key-ref customer/demo-1 --out /tmp/solum-demo/customer.keypair.json
```

Use `crypto keygen` + `--keypair` for Stage-1 evaluations. Ephemeral keys (`--ephemeral`) require `SOLUM_ALLOW_EPHEMERAL=1` and `config/profiles/dev-local.toml` — pilot profiles refuse them. See [customer/DEPLOYMENT-RUNBOOK.md](customer/DEPLOYMENT-RUNBOOK.md).

Audit `export` writes a HELIOS-oriented envelope only — this tree does not perform live HELIOS signing. Local UI/smoke of a tagged Solum: [Solum-Demo](https://github.com/SynapticFour/Solum-Demo).

Kenya is an evaluation profile, not a production candidate. Next: [ARCHITECTURE.md](ARCHITECTURE.md) · [FOR-EVALUATORS.md](FOR-EVALUATORS.md).
