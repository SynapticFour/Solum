//! `solum-sidecar` binary — HTTP wrap of Deployment `*_as`.
//!
//! Key custody matches the Phase‑C CLI: `--keys-dir` (CustomerHeld) by default
//! for evaluations; `--ephemeral` only behind `SOLUM_ALLOW_EPHEMERAL` + a profile
//! that allows `ephemeral_test`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use solum_sidecar::{serve, SidecarConfig};

#[derive(Debug, Parser)]
#[command(
    name = "solum-sidecar",
    version,
    about = "Solum HTTP sidecar for HMIS/EHR integrators (CustomerHeld --keys-dir; gated --ephemeral)"
)]
struct Cli {
    /// Bind address. Default loopback only — override deliberately for non-local use.
    #[arg(long, env = "SOLUM_SIDECAR_BIND", default_value = "127.0.0.1:8787")]
    bind: SocketAddr,

    #[arg(
        long,
        env = "SOLUM_PROFILE",
        default_value = "config/profiles/eu-ehds.toml"
    )]
    profile: PathBuf,

    #[arg(long, env = "SOLUM_AUDIT")]
    audit: PathBuf,

    #[arg(long = "consent-store", env = "SOLUM_CONSENT_STORE")]
    consent_store: PathBuf,

    /// Shared secret for `X-Solum-Sidecar-Token` (required). Not a GTM-1 capability.
    #[arg(long, env = "SOLUM_SIDECAR_TOKEN")]
    token: String,

    /// Directory of CustomerHeld keypair JSON files (`solum crypto keygen` layout).
    /// Required unless `--ephemeral`.
    #[arg(
        long = "keys-dir",
        env = "SOLUM_SIDECAR_KEYS_DIR",
        required_unless_present = "ephemeral"
    )]
    keys_dir: Option<PathBuf>,

    /// Dev-only ephemeral keys. Requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile
    /// that lists `ephemeral_test` (e.g. `dev-local.toml`).
    #[arg(long, default_value_t = false, conflicts_with = "keys_dir")]
    ephemeral: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let config = SidecarConfig {
        bind: cli.bind,
        profile: cli.profile,
        audit: cli.audit,
        consent_store: cli.consent_store,
        token: cli.token,
        keys_dir: cli.keys_dir,
        ephemeral: cli.ephemeral,
    };

    match serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}
