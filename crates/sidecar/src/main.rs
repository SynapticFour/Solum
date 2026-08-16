//! `solum-sidecar` binary — HTTP wrap of Deployment `*_as`.
//!
//! Key custody matches the Phase‑C CLI: `--keys-dir` (CustomerHeld) by default
//! for evaluations; `--ephemeral` only behind `SOLUM_ALLOW_EPHEMERAL` + a profile
//! that allows `ephemeral_test`.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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
    /// Required unless `--ephemeral` or `--wrapped-keys-dir`.
    #[arg(
        long = "keys-dir",
        env = "SOLUM_SIDECAR_KEYS_DIR",
        required_unless_present_any = ["ephemeral", "wrapped_keys_dir"]
    )]
    keys_dir: Option<PathBuf>,

    /// Dev-only ephemeral keys. Requires `SOLUM_ALLOW_EPHEMERAL=1` and a profile
    /// that lists `ephemeral_test` (e.g. `dev-local.toml`).
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["keys_dir", "wrapped_keys_dir"]
    )]
    ephemeral: bool,

    /// Directory of KMS-wrapped seed JSON (`solum crypto wrap-seed`).
    /// Requires build `--features aws-kms`.
    #[arg(
        long = "wrapped-keys-dir",
        env = "SOLUM_SIDECAR_WRAPPED_KEYS_DIR",
        conflicts_with_all = ["keys_dir", "ephemeral"]
    )]
    wrapped_keys_dir: Option<PathBuf>,

    /// Org-IAM mapping TOML (H2.2). When set, mutating routes require Bearer JWT
    /// and derive CAP_* from OIDC groups (body capability[] ignored).
    #[arg(long = "org-iam-config", env = "SOLUM_ORG_IAM_CONFIG")]
    org_iam_config: Option<PathBuf>,

    /// JWKS URL for org-IAM JWT verification.
    #[arg(long = "jwks-url", env = "SOLUM_ORG_IAM_JWKS_URL")]
    jwks_url: Option<String>,

    /// Local JWKS JSON file (alternative to --jwks-url).
    #[arg(long = "jwks-file", env = "SOLUM_ORG_IAM_JWKS_FILE")]
    jwks_file: Option<PathBuf>,

    /// Expected JWT issuer (required when --org-iam-config is set).
    #[arg(long = "oidc-issuer", env = "SOLUM_ORG_IAM_ISSUER")]
    oidc_issuer: Option<String>,

    /// JWT audience (required when --org-iam-config is set).
    #[arg(long = "oidc-audience", env = "SOLUM_ORG_IAM_AUDIENCE")]
    oidc_audience: Option<String>,

    /// Hospital IdP pack: entra | keycloak-hospital | smart-backend.
    /// Fills --org-iam-config (and default audience) from config/idp-profiles/ when unset.
    #[arg(long = "idp-profile", env = "SOLUM_IDP_PROFILE")]
    idp_profile: Option<String>,

    /// EHRbase base URL including `/ehrbase` context (H3.0 Track B). Opt-in.
    #[arg(long = "ehrbase-url", env = "SOLUM_EHRBASE_URL")]
    ehrbase_url: Option<String>,

    /// Optional OPT XML path for `POST /v1/cdr/template` (default: embedded pinned fixture).
    #[arg(long = "cdr-template-opt", env = "SOLUM_CDR_TEMPLATE_OPT")]
    cdr_template_opt: Option<PathBuf>,

    /// FHIR façade JSONL store (H3.1). Default beside consent store.
    #[arg(long = "fhir-store", env = "SOLUM_FHIR_STORE")]
    fhir_store: Option<PathBuf>,

    /// Subject bridge JSONL store (H3.3). Default beside consent store.
    #[arg(long = "subject-link-store", env = "SOLUM_SUBJECT_LINK_STORE")]
    subject_link_store: Option<PathBuf>,

    /// Dual-write dead-letter JSONL (H3.2 live webhook). Default beside consent store.
    #[arg(long = "dual-write-dead-letter", env = "SOLUM_DUAL_WRITE_DEAD_LETTER")]
    dual_write_dead_letter: Option<PathBuf>,
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

    let idp = match cli.idp_profile.as_deref() {
        Some(name) => match solum_identity::IdpProfile::load_named(Path::new("config"), name) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("solum-sidecar: {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };
    let org_iam_config = cli
        .org_iam_config
        .or_else(|| idp.as_ref().map(|p| p.org_iam_path()));
    let oidc_audience = cli.oidc_audience.or_else(|| {
        idp.as_ref()
            .map(|p| p.audience.clone())
            .filter(|s| !s.is_empty())
    });

    let config = SidecarConfig {
        bind: cli.bind,
        profile: cli.profile,
        audit: cli.audit,
        consent_store: cli.consent_store,
        token: cli.token,
        keys_dir: cli.keys_dir,
        ephemeral: cli.ephemeral,
        wrapped_keys_dir: cli.wrapped_keys_dir,
        org_iam_config,
        jwks_url: cli.jwks_url,
        jwks_file: cli.jwks_file,
        oidc_issuer: cli.oidc_issuer,
        oidc_audience,
        ehrbase_url: cli.ehrbase_url,
        cdr_template_opt: cli.cdr_template_opt,
        fhir_store: cli.fhir_store,
        subject_link_store: cli.subject_link_store,
        dual_write_dead_letter: cli.dual_write_dead_letter,
    };

    match serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}
