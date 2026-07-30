//! Solum CLI — jurisdiction check plus Deployment-backed consent / crypto / audit tools.
//!
//! Usage overview: see README.md “CLI usage”.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use solum_core::crypto::{
    Crypt4ghKeyProvider, CustomerHeldKeyProvider, EncryptedField, EphemeralTestKeyProvider, KeyRef,
};
use solum_core::{
    example_eu_runtime, query_consent_status, start_with_profile, ActorSource, Deployment,
    SolumActor, SolumError,
};

const EPHEMERAL_KEY_WARNING: &str = "\
⚠ Using EphemeralTestKeyProvider — keys are NOT persisted across runs
and are NOT suitable for real patient data. Production key custody
(CustomerHeld / HSM-backed) is not yet wired into the CLI.
The demo sidecar (*.ephemeral-keypair.json) contains raw private key
bytes in plaintext; 0600 permissions on Unix, no equivalent protection
on Windows.";

#[derive(Debug, Parser)]
#[command(name = "solum", version, about = "Solum clinical compliance CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate a jurisdiction profile against the declared runtime configuration.
    Check {
        #[arg(long, default_value = "config/profiles/eu-ehds.toml")]
        profile: PathBuf,
    },
    /// Consent grant / revoke / status against a persistent consent store.
    Consent {
        #[command(subcommand)]
        command: ConsentCmd,
    },
    /// Crypt4GH field encrypt / decrypt (demo keys only — see warning).
    Crypto {
        #[command(subcommand)]
        command: CryptoCmd,
    },
    /// Audit chain export and verification.
    Audit {
        #[command(subcommand)]
        command: AuditCmd,
    },
}

#[derive(Debug, Subcommand)]
enum ConsentCmd {
    /// Grant consent and emit a matching audit event.
    Grant {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        purpose: String,
        #[arg(long)]
        actor: String,
        /// Optional comma-separated consent data categories (not auth capabilities).
        #[arg(long, value_delimiter = ',')]
        scope: Vec<String>,
        /// GTM-1 authorization capability (repeatable). Fail-closed: omit → empty
        /// scopes → denied. Distinct from `--scope` (consent data categories).
        #[arg(long = "capability", action = clap::ArgAction::Append)]
        capability: Vec<String>,
    },
    /// Revoke consent and emit a matching audit event.
    Revoke {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        purpose: String,
        #[arg(long)]
        actor: String,
        /// GTM-1 authorization capability (repeatable). Fail-closed: omit → denied.
        #[arg(long = "capability", action = clap::ArgAction::Append)]
        capability: Vec<String>,
    },
    /// Print granted / revoked / unknown (read-only; no audit path required).
    Status {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        purpose: String,
    },
}

#[derive(Debug, Subcommand)]
enum CryptoCmd {
    /// Encrypt a file into an EncryptedField JSON document.
    Encrypt {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        #[arg(long)]
        category: String,
        #[arg(long)]
        key_ref: String,
        #[arg(long)]
        actor: String,
        /// GTM-1 authorization capability (repeatable). Fail-closed: omit → denied.
        #[arg(long = "capability", action = clap::ArgAction::Append)]
        capability: Vec<String>,
        #[arg(long)]
        r#in: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Decrypt an EncryptedField JSON document to a plaintext file.
    Decrypt {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        #[arg(long)]
        key_ref: String,
        #[arg(long)]
        actor: String,
        /// GTM-1 authorization capability (repeatable). Fail-closed: omit → denied.
        #[arg(long = "capability", action = clap::ArgAction::Append)]
        capability: Vec<String>,
        #[arg(long)]
        r#in: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum AuditCmd {
    /// Export the audit chain as a HELIOS-oriented JSON envelope.
    Export {
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify the audit hash chain; prints "ok" or a ChainBroken reason.
    Verify {
        #[arg(long)]
        audit: PathBuf,
    },
}

/// Demo-only key material written beside EncryptedField JSON so CLI encrypt →
/// decrypt can round-trip across process boundaries.
///
/// `EphemeralTestKeyProvider` has no public import API (and we do not extend
/// `solum-crypto` here). Encrypt generates via that provider and persists the
/// returned key bytes; decrypt rehydrates them into `CustomerHeldKeyProvider`
/// for the decrypt call only. This is still not production custody — see
/// [`EPHEMERAL_KEY_WARNING`].
#[derive(Debug, Serialize, Deserialize)]
struct DemoEphemeralKeypair {
    key_ref: String,
    pubkey: Vec<u8>,
    privkey: Vec<u8>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run(cli: Cli) -> Result<(), ExitCode> {
    match cli.command {
        Commands::Check { profile } => cmd_check(profile),
        Commands::Consent { command } => cmd_consent(command),
        Commands::Crypto { command } => cmd_crypto(command),
        Commands::Audit { command } => cmd_audit(command),
    }
}

fn runtime_config() -> solum_core::profiles::RuntimeConfig {
    let mut runtime = example_eu_runtime();
    if let Ok(region) = env::var("SOLUM_STORAGE_REGION") {
        runtime.storage_region = region;
    }
    runtime
}

fn fail(err: impl std::fmt::Display) -> ExitCode {
    eprintln!("fatal: {err}");
    ExitCode::FAILURE
}

fn fail_usage(err: impl std::fmt::Display) -> ExitCode {
    eprintln!("fatal: {err}");
    ExitCode::from(2)
}

fn cmd_check(profile: PathBuf) -> Result<(), ExitCode> {
    let runtime = runtime_config();
    match start_with_profile(&profile, &runtime) {
        Ok(p) => {
            println!(
                "ok: profile '{}' (jurisdiction {}) matches runtime configuration",
                p.meta.profile, p.meta.jurisdiction
            );
            Ok(())
        }
        Err(e) => Err(fail(e)),
    }
}

fn open_deployment<P: Crypt4ghKeyProvider>(
    profile: &Path,
    audit: &Path,
    consent_store: &Path,
    keys: P,
) -> Result<Deployment<P>, ExitCode> {
    Deployment::open(profile, &runtime_config(), audit, consent_store, keys).map_err(fail)
}

/// CLI actor for GTM-1 `*_as` paths.
///
/// `LocalDev` keeps `to_audit_string()` identical to the pre-GTM CLI `&str`
/// actor (e.g. `"practitioner/7"`), so audit trails stay comparable. Omit
/// `--capability` → empty scopes → fail-closed denial (option A).
fn cli_actor(subject_id: String, capabilities: Vec<String>) -> SolumActor {
    SolumActor {
        subject_id,
        display: None,
        source: ActorSource::LocalDev,
        scopes: capabilities,
    }
}

fn cmd_consent(command: ConsentCmd) -> Result<(), ExitCode> {
    match command {
        ConsentCmd::Grant {
            profile,
            audit,
            consent_store,
            subject,
            purpose,
            actor,
            scope,
            capability,
        } => {
            let mut deployment = open_deployment(
                &profile,
                &audit,
                &consent_store,
                EphemeralTestKeyProvider::new(),
            )?;
            let actor = cli_actor(actor, capability);
            let record = deployment
                .grant_consent_as(&subject, &purpose, scope, &actor)
                .map_err(fail)?;
            print_json(&record)?;
            Ok(())
        }
        ConsentCmd::Revoke {
            profile,
            audit,
            consent_store,
            subject,
            purpose,
            actor,
            capability,
        } => {
            let mut deployment = open_deployment(
                &profile,
                &audit,
                &consent_store,
                EphemeralTestKeyProvider::new(),
            )?;
            let actor = cli_actor(actor, capability);
            let record = deployment
                .revoke_consent_as(&subject, &purpose, &actor)
                .map_err(fail)?;
            print_json(&record)?;
            Ok(())
        }
        ConsentCmd::Status {
            profile,
            consent_store,
            subject,
            purpose,
        } => {
            // See `query_consent_status` docs: no audit path on purpose.
            let status = query_consent_status(
                &profile,
                &runtime_config(),
                &consent_store,
                &subject,
                &purpose,
            )
            .map_err(fail)?;
            println!("{status}");
            Ok(())
        }
    }
}

fn cmd_crypto(command: CryptoCmd) -> Result<(), ExitCode> {
    eprintln!("{EPHEMERAL_KEY_WARNING}");
    match command {
        CryptoCmd::Encrypt {
            profile,
            audit,
            consent_store,
            category,
            key_ref,
            actor,
            capability,
            r#in,
            out,
        } => {
            let key_ref = KeyRef::new(key_ref);
            let mut keys = EphemeralTestKeyProvider::new();
            let (pubkey, privkey) = keys
                .generate_test_keypair(key_ref.clone())
                .map_err(|e| fail(SolumError::Message(e.to_string())))?;

            let plaintext = fs::read(&r#in)
                .map_err(|e| fail_usage(format!("failed to read --in {}: {e}", r#in.display())))?;

            let mut deployment = open_deployment(&profile, &audit, &consent_store, keys)?;
            let actor = cli_actor(actor, capability);
            let field = deployment
                .encrypt_field_as(&category, &plaintext, &key_ref, &actor)
                .map_err(fail)?;

            write_json(&out, &field)?;
            write_json(
                &demo_keypair_sidecar(&out),
                &DemoEphemeralKeypair {
                    key_ref: key_ref.id,
                    pubkey,
                    privkey,
                },
            )?;
            // Restrict sidecar to owner read/write on Unix (raw private key bytes).
            // Windows: no POSIX permission equivalent set here — demo-only sidecar,
            // do not use for real key custody on any platform.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let sidecar = demo_keypair_sidecar(&out);
                let mut perms = fs::metadata(&sidecar)
                    .map_err(|e| fail(format!("stat {}: {e}", sidecar.display())))?
                    .permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&sidecar, perms)
                    .map_err(|e| fail(format!("chmod {}: {e}", sidecar.display())))?;
            }
            Ok(())
        }
        CryptoCmd::Decrypt {
            profile,
            audit,
            consent_store,
            key_ref,
            actor,
            capability,
            r#in,
            out,
        } => {
            // Rehydrate demo key bytes into CustomerHeldKeyProvider: EphemeralTestKeyProvider
            // cannot import a prior generate_test_keypair result (no public register API;
            // solum-crypto is intentionally not extended for this CLI).
            let sidecar = demo_keypair_sidecar(&r#in);
            if !sidecar.exists() {
                return Err(fail(format!(
                    "demo key sidecar {} missing — run crypto encrypt first in this workspace",
                    sidecar.display()
                )));
            }
            let demo: DemoEphemeralKeypair = read_json(&sidecar)?;
            if demo.key_ref != key_ref {
                return Err(fail(format!(
                    "key-ref '{key_ref}' does not match demo sidecar key '{}'",
                    demo.key_ref
                )));
            }
            let key_ref = KeyRef::new(key_ref);
            let mut keys = CustomerHeldKeyProvider::new();
            keys.register_customer_keypair(key_ref.clone(), demo.pubkey, demo.privkey)
                .map_err(|e| fail(SolumError::Message(e.to_string())))?;

            let field: EncryptedField = read_json(&r#in)?;
            let mut deployment = open_deployment(&profile, &audit, &consent_store, keys)?;
            let actor = cli_actor(actor, capability);
            let plaintext = deployment
                .decrypt_field_as(&field, &key_ref, &actor)
                .map_err(fail)?;
            fs::write(&out, plaintext)
                .map_err(|e| fail(format!("failed to write --out {}: {e}", out.display())))?;
            Ok(())
        }
    }
}

fn cmd_audit(command: AuditCmd) -> Result<(), ExitCode> {
    match command {
        AuditCmd::Export { audit, out } => {
            let store = solum_core::audit::FileAuditStore::open(&audit)
                .map_err(|e| fail(format!("audit store: {e}")))?;
            let json = store
                .export_helios_json()
                .map_err(|e| fail(format!("audit export: {e}")))?;
            fs::write(&out, json)
                .map_err(|e| fail(format!("failed to write --out {}: {e}", out.display())))?;
            Ok(())
        }
        AuditCmd::Verify { audit } => {
            let store = solum_core::audit::FileAuditStore::open(&audit)
                .map_err(|e| fail(format!("audit store: {e}")))?;
            match store.verify_chain() {
                Ok(()) => {
                    println!("ok");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{e}");
                    Err(ExitCode::FAILURE)
                }
            }
        }
    }
}

fn demo_keypair_sidecar(field_path: &Path) -> PathBuf {
    let mut os = field_path.as_os_str().to_owned();
    os.push(".ephemeral-keypair.json");
    PathBuf::from(os)
}

fn print_json(value: &impl Serialize) -> Result<(), ExitCode> {
    let json =
        serde_json::to_string_pretty(value).map_err(|e| fail(format!("JSON serialize: {e}")))?;
    println!("{json}");
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), ExitCode> {
    let json =
        serde_json::to_string_pretty(value).map_err(|e| fail(format!("JSON serialize: {e}")))?;
    fs::write(path, json).map_err(|e| fail(format!("failed to write {}: {e}", path.display())))?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ExitCode> {
    let raw = fs::read_to_string(path)
        .map_err(|e| fail(format!("failed to read {}: {e}", path.display())))?;
    serde_json::from_str(&raw).map_err(|e| fail(format!("JSON parse {}: {e}", path.display())))
}
