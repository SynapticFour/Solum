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
    generate_operator_keypair, Crypt4ghKeyProvider, CustomerHeldKeyProvider, EncryptedField,
    EphemeralTestKeyProvider, KeyCustody, KeyRef,
};
use solum_core::{
    example_eu_runtime, query_consent_status, start_with_profile, ActorSource, Deployment,
    SolumActor, SolumError,
};

const EPHEMERAL_KEY_WARNING: &str = "\
⚠ Using EphemeralTestKeyProvider — keys are NOT persisted across runs
and are NOT suitable for real patient data or paid evaluations.
Requires SOLUM_ALLOW_EPHEMERAL=1 and a profile that allows ephemeral_test
(e.g. config/profiles/dev-local.toml). Pilot profiles (eu-ehds, kenya-dpa)
refuse EphemeralTest custody at startup.
The demo sidecar (*.ephemeral-keypair.json) contains raw private key
bytes in plaintext; 0600 permissions on Unix, no equivalent protection
on Windows.";

const CUSTOMER_HELD_KEY_NOTE: &str = "\
Using CustomerHeld key material from --keypair (operator-supplied file).
Solum does not mint these keys during encrypt; protect the keypair file
as you would other secrets (0600 on Unix recommended).";

#[cfg_attr(not(feature = "aws-kms"), allow(dead_code))]
const AWS_KMS_KEY_NOTE: &str = "\
Using AWS KMS-wrapped Crypt4GH seed (CustomerHeld custody; provider=aws-kms).
Seed is unwrapped into process memory (ZeroizeOnDrop) — envelope encryption, not an HSM/TEE.
Build with --features aws-kms; configure AWS_REGION and credentials (or instance role).";

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
    /// Crypt4GH field encrypt / decrypt (CustomerHeld --keypair by default).
    Crypto {
        #[command(subcommand)]
        command: CryptoCmd,
    },
    /// Audit chain export and verification.
    Audit {
        #[command(subcommand)]
        command: AuditCmd,
    },
    /// H3.2 migration helpers (fhir-import inventory + dual-write dead-letter).
    Migrate {
        #[command(subcommand)]
        command: MigrateCmd,
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
    /// Generate an operator keypair file for CustomerHeld registration.
    ///
    /// Writes JSON `{key_ref, pubkey, privkey}` for later `--keypair` use.
    /// Material is operator-controlled; Solum does not retain it after write.
    Keygen {
        #[arg(long)]
        key_ref: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Wrap a CustomerHeld private seed under AWS KMS (`--features aws-kms`).
    ///
    /// Writes JSON `{key_ref, kms_key_id, wrapped_seed}` for `--wrapped-keypair`.
    WrapSeed {
        #[arg(long)]
        key_ref: String,
        /// KMS key id, alias, or ARN.
        #[arg(long)]
        kms_key_id: String,
        /// Plaintext keypair from `crypto keygen` (privkey provides the seed).
        #[arg(long)]
        keypair: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
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
        /// Data subject whose consent must be active for this category.
        #[arg(long)]
        subject: String,
        /// Consent purpose (must be granted and cover `--category`).
        #[arg(long)]
        purpose: String,
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
        /// CustomerHeld Crypt4GH keypair JSON from `crypto keygen`.
        #[arg(long, required_unless_present_any = ["ephemeral", "wrapped_keypair"])]
        keypair: Option<PathBuf>,
        /// Dev-only ephemeral keys. Requires `SOLUM_ALLOW_EPHEMERAL=1` and a
        /// profile that lists `ephemeral_test` (e.g. `dev-local.toml`).
        #[arg(long, default_value_t = false, conflicts_with_all = ["keypair", "wrapped_keypair"])]
        ephemeral: bool,
        /// AWS KMS-wrapped seed JSON from `crypto wrap-seed` (requires `--features aws-kms`).
        #[arg(long, conflicts_with_all = ["keypair", "ephemeral"])]
        wrapped_keypair: Option<PathBuf>,
    },
    /// Decrypt an EncryptedField JSON document to a plaintext file.
    Decrypt {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        consent_store: PathBuf,
        /// Data subject whose consent must be active for the field category.
        #[arg(long)]
        subject: String,
        /// Consent purpose (must be granted and cover the field category).
        #[arg(long)]
        purpose: String,
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
        /// CustomerHeld Crypt4GH keypair JSON (required unless `--ephemeral` / `--wrapped-keypair`).
        #[arg(long, required_unless_present_any = ["ephemeral", "wrapped_keypair"])]
        keypair: Option<PathBuf>,
        /// Dev-only ephemeral keys. Requires `SOLUM_ALLOW_EPHEMERAL=1` and a
        /// profile that lists `ephemeral_test` (e.g. `dev-local.toml`).
        #[arg(long, default_value_t = false, conflicts_with_all = ["keypair", "wrapped_keypair"])]
        ephemeral: bool,
        /// AWS KMS-wrapped seed JSON from `crypto wrap-seed` (requires `--features aws-kms`).
        #[arg(long, conflicts_with_all = ["keypair", "ephemeral"])]
        wrapped_keypair: Option<PathBuf>,
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

#[derive(Debug, Subcommand)]
enum MigrateCmd {
    /// Parse a FHIR Bundle/resource file and print idempotent import inventory (H3.2).
    /// Does not call EHRbase by itself — feed listed ids through sidecar `/v1/fhir/*`.
    FhirImport {
        #[arg(long)]
        bundle: PathBuf,
        /// Optional JSONL of already-imported `ResourceType/id` keys (skip duplicates).
        #[arg(long)]
        seen: Option<PathBuf>,
        /// Write inventory JSONL of resources to import.
        #[arg(long)]
        out: PathBuf,
    },
    /// Dual-write stub: append failed mirror payload to a dead-letter JSONL (never silent).
    DualWriteStub {
        #[arg(long)]
        payload: PathBuf,
        #[arg(long)]
        dead_letter: PathBuf,
        /// Simulated failure reason recorded in the dead-letter row.
        #[arg(long, default_value = "dual_write_failed")]
        reason: String,
    },
}

/// Operator / CustomerHeld key material on disk (JSON).
///
/// Same layout as the legacy demo ephemeral sidecar so existing fixtures remain
/// readable; custody mode is determined by CLI flags + runtime config, not the
/// file extension.
#[derive(Debug, Serialize, Deserialize)]
struct KeypairFile {
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
        Commands::Migrate { command } => cmd_migrate(command),
    }
}

fn runtime_config(custody: KeyCustody) -> solum_core::profiles::RuntimeConfig {
    let mut runtime = example_eu_runtime();
    if let Ok(region) = env::var("SOLUM_STORAGE_REGION") {
        runtime.storage_region = region;
    }
    runtime.key_management.provider = match &custody {
        KeyCustody::CustomerHeld => Some("customer-held-file".into()),
        KeyCustody::EphemeralTest => Some("ephemeral-test".into()),
        KeyCustody::OperatorHeld => runtime.key_management.provider,
    };
    runtime.key_management.custody = custody;
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

fn ephemeral_env_allowed() -> bool {
    match env::var("SOLUM_ALLOW_EPHEMERAL") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

fn require_ephemeral_gate() -> Result<(), ExitCode> {
    if ephemeral_env_allowed() {
        return Ok(());
    }
    Err(fail_usage(
        "ephemeral crypto requires SOLUM_ALLOW_EPHEMERAL=1 (or true/yes). \
         Paid evaluations and pilots must use --keypair (CustomerHeld). \
         See docs/customer/DEPLOYMENT-RUNBOOK.md §4.",
    ))
}

fn cmd_check(profile: PathBuf) -> Result<(), ExitCode> {
    // Check uses CustomerHeld runtime (matches pilot profiles). Override custody
    // via SOLUM_KEY_CUSTODY=ephemeral_test only when exercising the refuse path.
    let custody = match env::var("SOLUM_KEY_CUSTODY") {
        Ok(v) if v.eq_ignore_ascii_case("ephemeral_test") => KeyCustody::EphemeralTest,
        Ok(v) if v.eq_ignore_ascii_case("operator_held") => KeyCustody::OperatorHeld,
        _ => KeyCustody::CustomerHeld,
    };
    let runtime = runtime_config(custody);
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
    custody: KeyCustody,
) -> Result<Deployment<P>, ExitCode> {
    open_deployment_with_provider(profile, audit, consent_store, keys, custody, None)
}

fn open_deployment_with_provider<P: Crypt4ghKeyProvider>(
    profile: &Path,
    audit: &Path,
    consent_store: &Path,
    keys: P,
    custody: KeyCustody,
    provider_override: Option<&str>,
) -> Result<Deployment<P>, ExitCode> {
    let mut runtime = runtime_config(custody);
    if let Some(p) = provider_override {
        runtime.key_management.provider = Some(p.into());
    }
    Deployment::open(profile, &runtime, audit, consent_store, keys).map_err(fail)
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
    // Consent does not touch Crypt4GH keys — use an empty CustomerHeld registry
    // so pilot profiles never boot under EphemeralTest custody by accident.
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
                CustomerHeldKeyProvider::new(),
                KeyCustody::CustomerHeld,
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
                CustomerHeldKeyProvider::new(),
                KeyCustody::CustomerHeld,
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
                &runtime_config(KeyCustody::CustomerHeld),
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

fn load_keypair_file(path: &Path, expected_key_ref: &str) -> Result<KeypairFile, ExitCode> {
    let kp: KeypairFile = read_json(path)?;
    if kp.key_ref != expected_key_ref {
        return Err(fail(format!(
            "key-ref '{expected_key_ref}' does not match keypair file key '{}'",
            kp.key_ref
        )));
    }
    Ok(kp)
}

fn customer_provider_from_file(
    path: &Path,
    expected_key_ref: &str,
) -> Result<(CustomerHeldKeyProvider, KeyRef), ExitCode> {
    let file = load_keypair_file(path, expected_key_ref)?;
    let key_ref = KeyRef::new(file.key_ref);
    let mut keys = CustomerHeldKeyProvider::new();
    keys.register_customer_keypair(key_ref.clone(), file.pubkey, file.privkey)
        .map_err(|e| fail(SolumError::Message(e.to_string())))?;
    Ok((keys, key_ref))
}

fn chmod_owner_rw(path: &Path) -> Result<(), ExitCode> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .map_err(|e| fail(format!("stat {}: {e}", path.display())))?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)
            .map_err(|e| fail(format!("chmod {}: {e}", path.display())))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn cmd_crypto(command: CryptoCmd) -> Result<(), ExitCode> {
    match command {
        CryptoCmd::Keygen { key_ref, out } => {
            let (pubkey, privkey) = generate_operator_keypair()
                .map_err(|e| fail(SolumError::Message(e.to_string())))?;
            write_json(
                &out,
                &KeypairFile {
                    key_ref,
                    pubkey,
                    privkey,
                },
            )?;
            chmod_owner_rw(&out)?;
            eprintln!(
                "wrote CustomerHeld keypair to {} (protect this file; not an HSM)",
                out.display()
            );
            Ok(())
        }
        CryptoCmd::WrapSeed {
            key_ref,
            kms_key_id,
            keypair,
            out,
        } => cmd_crypto_wrap_seed(key_ref, kms_key_id, keypair, out),
        CryptoCmd::Encrypt {
            profile,
            audit,
            consent_store,
            category,
            subject,
            purpose,
            key_ref,
            actor,
            capability,
            r#in,
            out,
            keypair,
            ephemeral,
            wrapped_keypair,
        } => {
            if ephemeral {
                require_ephemeral_gate()?;
                eprintln!("{EPHEMERAL_KEY_WARNING}");
                let key_ref = KeyRef::new(key_ref);
                let mut keys = EphemeralTestKeyProvider::new();
                let (pubkey, privkey) = keys
                    .generate_test_keypair(key_ref.clone())
                    .map_err(|e| fail(SolumError::Message(e.to_string())))?;

                let plaintext = fs::read(&r#in).map_err(|e| {
                    fail_usage(format!("failed to read --in {}: {e}", r#in.display()))
                })?;

                let mut deployment = open_deployment(
                    &profile,
                    &audit,
                    &consent_store,
                    keys,
                    KeyCustody::EphemeralTest,
                )?;
                let actor = cli_actor(actor, capability);
                let field = deployment
                    .encrypt_field_as(&category, &plaintext, &key_ref, &actor, &subject, &purpose)
                    .map_err(fail)?;

                write_json(&out, &field)?;
                let sidecar = demo_keypair_sidecar(&out);
                write_json(
                    &sidecar,
                    &KeypairFile {
                        key_ref: key_ref.id,
                        pubkey,
                        privkey,
                    },
                )?;
                chmod_owner_rw(&sidecar)?;
                Ok(())
            } else if let Some(wrapped_path) = wrapped_keypair {
                cmd_crypto_encrypt_wrapped(
                    profile,
                    audit,
                    consent_store,
                    category,
                    subject,
                    purpose,
                    key_ref,
                    actor,
                    capability,
                    r#in,
                    out,
                    wrapped_path,
                )
            } else {
                let keypair_path = keypair.ok_or_else(|| {
                    fail_usage("--keypair is required (or --ephemeral / --wrapped-keypair)")
                })?;
                eprintln!("{CUSTOMER_HELD_KEY_NOTE}");
                let (keys, key_ref) = customer_provider_from_file(&keypair_path, &key_ref)?;
                let plaintext = fs::read(&r#in).map_err(|e| {
                    fail_usage(format!("failed to read --in {}: {e}", r#in.display()))
                })?;

                let mut deployment = open_deployment(
                    &profile,
                    &audit,
                    &consent_store,
                    keys,
                    KeyCustody::CustomerHeld,
                )?;
                let actor = cli_actor(actor, capability);
                let field = deployment
                    .encrypt_field_as(&category, &plaintext, &key_ref, &actor, &subject, &purpose)
                    .map_err(fail)?;
                write_json(&out, &field)?;
                Ok(())
            }
        }
        CryptoCmd::Decrypt {
            profile,
            audit,
            consent_store,
            subject,
            purpose,
            key_ref,
            actor,
            capability,
            r#in,
            out,
            keypair,
            ephemeral,
            wrapped_keypair,
        } => {
            if ephemeral {
                require_ephemeral_gate()?;
                eprintln!("{EPHEMERAL_KEY_WARNING}");
                let sidecar = demo_keypair_sidecar(&r#in);
                if !sidecar.exists() {
                    return Err(fail(format!(
                        "demo key sidecar {} missing — run crypto encrypt --ephemeral first",
                        sidecar.display()
                    )));
                }
                let demo = load_keypair_file(&sidecar, &key_ref)?;
                let key_ref = KeyRef::new(demo.key_ref);
                let mut keys = CustomerHeldKeyProvider::new();
                keys.register_customer_keypair(key_ref.clone(), demo.pubkey, demo.privkey)
                    .map_err(|e| fail(SolumError::Message(e.to_string())))?;

                // Rehydrate via CustomerHeld provider, but declare EphemeralTest
                // custody so pilot profiles still refuse this path at startup.
                let field: EncryptedField = read_json(&r#in)?;
                let mut deployment = open_deployment(
                    &profile,
                    &audit,
                    &consent_store,
                    keys,
                    KeyCustody::EphemeralTest,
                )?;
                let actor = cli_actor(actor, capability);
                let plaintext = deployment
                    .decrypt_field_as(&field, &key_ref, &actor, &subject, &purpose)
                    .map_err(fail)?;
                fs::write(&out, plaintext)
                    .map_err(|e| fail(format!("failed to write --out {}: {e}", out.display())))?;
                Ok(())
            } else if let Some(wrapped_path) = wrapped_keypair {
                cmd_crypto_decrypt_wrapped(
                    profile,
                    audit,
                    consent_store,
                    subject,
                    purpose,
                    key_ref,
                    actor,
                    capability,
                    r#in,
                    out,
                    wrapped_path,
                )
            } else {
                let keypair_path = keypair.ok_or_else(|| {
                    fail_usage("--keypair is required (or --ephemeral / --wrapped-keypair)")
                })?;
                eprintln!("{CUSTOMER_HELD_KEY_NOTE}");
                let (keys, key_ref) = customer_provider_from_file(&keypair_path, &key_ref)?;
                let field: EncryptedField = read_json(&r#in)?;
                let mut deployment = open_deployment(
                    &profile,
                    &audit,
                    &consent_store,
                    keys,
                    KeyCustody::CustomerHeld,
                )?;
                let actor = cli_actor(actor, capability);
                let plaintext = deployment
                    .decrypt_field_as(&field, &key_ref, &actor, &subject, &purpose)
                    .map_err(fail)?;
                fs::write(&out, plaintext)
                    .map_err(|e| fail(format!("failed to write --out {}: {e}", out.display())))?;
                Ok(())
            }
        }
    }
}

fn require_aws_kms_feature() -> Result<(), ExitCode> {
    #[cfg(feature = "aws-kms")]
    {
        Ok(())
    }
    #[cfg(not(feature = "aws-kms"))]
    {
        Err(fail_usage(
            "AWS KMS CLI path requires rebuilding with --features aws-kms \
             (e.g. cargo run -p solum-core --features aws-kms -- crypto wrap-seed …)",
        ))
    }
}

fn cmd_crypto_wrap_seed(
    key_ref: String,
    kms_key_id: String,
    keypair: PathBuf,
    out: PathBuf,
) -> Result<(), ExitCode> {
    require_aws_kms_feature()?;
    #[cfg(feature = "aws-kms")]
    {
        use solum_core::crypto::aws_kms::{AwsKmsKeyProvider, WrappedSeedFile};
        eprintln!("{AWS_KMS_KEY_NOTE}");
        let file = load_keypair_file(&keypair, &key_ref)?;
        let seed = &file.privkey;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| fail(format!("tokio runtime: {e}")))?;
        let wrapped = rt
            .block_on(async {
                let client = solum_core::crypto::aws_kms::client_from_env()?;
                AwsKmsKeyProvider::wrap_seed(&client, &kms_key_id, seed).await
            })
            .map_err(|e| fail(e))?;
        let doc = WrappedSeedFile {
            key_ref,
            kms_key_id,
            wrapped_seed: wrapped,
        };
        doc.write(&out).map_err(fail)?;
        eprintln!(
            "wrote KMS-wrapped seed to {} (0600 on Unix; not an HSM export)",
            out.display()
        );
        Ok(())
    }
    #[cfg(not(feature = "aws-kms"))]
    {
        let _ = (key_ref, kms_key_id, keypair, out);
        unreachable!()
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_crypto_encrypt_wrapped(
    profile: PathBuf,
    audit: PathBuf,
    consent_store: PathBuf,
    category: String,
    subject: String,
    purpose: String,
    key_ref: String,
    actor: String,
    capability: Vec<String>,
    r#in: PathBuf,
    out: PathBuf,
    wrapped_path: PathBuf,
) -> Result<(), ExitCode> {
    require_aws_kms_feature()?;
    #[cfg(feature = "aws-kms")]
    {
        use solum_core::crypto::aws_kms::{AwsKmsKeyProvider, WrappedSeedFile};
        eprintln!("{AWS_KMS_KEY_NOTE}");
        let file = WrappedSeedFile::load(&wrapped_path).map_err(fail)?;
        if file.key_ref != key_ref {
            return Err(fail(format!(
                "key-ref '{key_ref}' does not match wrapped file key '{}'",
                file.key_ref
            )));
        }
        let plaintext = fs::read(&r#in)
            .map_err(|e| fail_usage(format!("failed to read --in {}: {e}", r#in.display())))?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| fail(format!("tokio runtime: {e}")))?;
        let keys = rt
            .block_on(async {
                let client = solum_core::crypto::aws_kms::client_from_env()?;
                AwsKmsKeyProvider::from_wrapped_seed(
                    &client,
                    KeyRef::new(file.key_ref),
                    &file.wrapped_seed,
                )
                .await
            })
            .map_err(fail)?;
        let key_ref = KeyRef::new(key_ref);
        let mut deployment = open_deployment_with_provider(
            &profile,
            &audit,
            &consent_store,
            keys,
            KeyCustody::CustomerHeld,
            Some("aws-kms"),
        )?;
        let actor = cli_actor(actor, capability);
        let field = deployment
            .encrypt_field_as(&category, &plaintext, &key_ref, &actor, &subject, &purpose)
            .map_err(fail)?;
        write_json(&out, &field)?;
        Ok(())
    }
    #[cfg(not(feature = "aws-kms"))]
    {
        let _ = (
            profile,
            audit,
            consent_store,
            category,
            subject,
            purpose,
            key_ref,
            actor,
            capability,
            r#in,
            out,
            wrapped_path,
        );
        unreachable!()
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_crypto_decrypt_wrapped(
    profile: PathBuf,
    audit: PathBuf,
    consent_store: PathBuf,
    subject: String,
    purpose: String,
    key_ref: String,
    actor: String,
    capability: Vec<String>,
    r#in: PathBuf,
    out: PathBuf,
    wrapped_path: PathBuf,
) -> Result<(), ExitCode> {
    require_aws_kms_feature()?;
    #[cfg(feature = "aws-kms")]
    {
        use solum_core::crypto::aws_kms::{AwsKmsKeyProvider, WrappedSeedFile};
        eprintln!("{AWS_KMS_KEY_NOTE}");
        let file = WrappedSeedFile::load(&wrapped_path).map_err(fail)?;
        if file.key_ref != key_ref {
            return Err(fail(format!(
                "key-ref '{key_ref}' does not match wrapped file key '{}'",
                file.key_ref
            )));
        }
        let field: EncryptedField = read_json(&r#in)?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| fail(format!("tokio runtime: {e}")))?;
        let keys = rt
            .block_on(async {
                let client = solum_core::crypto::aws_kms::client_from_env()?;
                AwsKmsKeyProvider::from_wrapped_seed(
                    &client,
                    KeyRef::new(file.key_ref),
                    &file.wrapped_seed,
                )
                .await
            })
            .map_err(fail)?;
        let key_ref = KeyRef::new(key_ref);
        let mut deployment = open_deployment_with_provider(
            &profile,
            &audit,
            &consent_store,
            keys,
            KeyCustody::CustomerHeld,
            Some("aws-kms"),
        )?;
        let actor = cli_actor(actor, capability);
        let plaintext = deployment
            .decrypt_field_as(&field, &key_ref, &actor, &subject, &purpose)
            .map_err(fail)?;
        fs::write(&out, plaintext)
            .map_err(|e| fail(format!("failed to write --out {}: {e}", out.display())))?;
        Ok(())
    }
    #[cfg(not(feature = "aws-kms"))]
    {
        let _ = (
            profile,
            audit,
            consent_store,
            subject,
            purpose,
            key_ref,
            actor,
            capability,
            r#in,
            out,
            wrapped_path,
        );
        unreachable!()
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

fn cmd_migrate(command: MigrateCmd) -> Result<(), ExitCode> {
    match command {
        MigrateCmd::FhirImport { bundle, seen, out } => {
            let doc = solum_core::load_fhir_json(&bundle)
                .map_err(|e| fail(format!("load bundle: {e}")))?;
            let resources = solum_core::extract_fhir_resources(&doc)
                .map_err(|e| fail(format!("extract: {e}")))?;
            let mut seen_keys = std::collections::HashSet::new();
            if let Some(path) = seen {
                if path.exists() {
                    for line in fs::read_to_string(&path)
                        .map_err(|e| fail(e.to_string()))?
                        .lines()
                    {
                        let t = line.trim();
                        if !t.is_empty() {
                            seen_keys.insert(t.to_string());
                        }
                    }
                }
            }
            let mut lines = Vec::new();
            let mut imported = 0usize;
            let mut skipped = 0usize;
            for res in resources {
                let key = solum_core::resource_idempotency_key(&res);
                if seen_keys.contains(&key) {
                    skipped += 1;
                    continue;
                }
                lines.push(
                    serde_json::to_string(&serde_json::json!({
                        "key": key,
                        "resource": res
                    }))
                    .map_err(|e| fail(e.to_string()))?,
                );
                imported += 1;
            }
            fs::write(
                &out,
                lines.join("\n") + if lines.is_empty() { "" } else { "\n" },
            )
            .map_err(|e| fail(format!("write {}: {e}", out.display())))?;
            println!(
                "imported_candidates={imported} skipped_seen={skipped} out={}",
                out.display()
            );
            Ok(())
        }
        MigrateCmd::DualWriteStub {
            payload,
            dead_letter,
            reason,
        } => {
            let body: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(&payload).map_err(|e| fail(e.to_string()))?,
            )
            .map_err(|e| fail(e.to_string()))?;
            let row = solum_core::dead_letter_row(&reason, &body);
            solum_core::append_dead_letter(&dead_letter, &row)
                .map_err(|e| fail(format!("dead-letter: {e}")))?;
            println!("dead_letter_appended={}", dead_letter.display());
            Ok(())
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
