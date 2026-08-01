//! `solum-sidecar` binary — HTTP wrap of Deployment `*_as` (demo keys only).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use solum_sidecar::{serve, SidecarConfig, EPHEMERAL_KEY_WARNING};

#[derive(Debug, Parser)]
#[command(
    name = "solum-sidecar",
    version,
    about = "Solum HTTP sidecar for HMIS/EHR integrators (Stage 1 — ephemeral keys only)"
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
    eprintln!("{EPHEMERAL_KEY_WARNING}");

    let config = SidecarConfig {
        bind: cli.bind,
        profile: cli.profile,
        audit: cli.audit,
        consent_store: cli.consent_store,
        token: cli.token,
    };

    match serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}
