//! HTTP sidecar wrapping [`solum_core::Deployment`]'s capability-checked `*_as`
//! methods for non-Rust HMIS/EHR integrators (PHP / Python / Java, …).
//!
//! # Why axum
//!
//! Axum is already the HTTP stack used on the Ferrum side (auth middleware /
//! passport surfaces). Reusing it keeps dependency posture and request-handler
//! idioms aligned with the portfolio instead of introducing a second framework
//! (actix, warp, …) solely for this binary.
//!
//! # Key custody — CustomerHeld default, ephemeral gated
//!
//! Same posture as the Phase‑C CLI:
//! - **`--keys-dir`** — load operator keypair JSON files (`solum crypto keygen`
//!   layout) into [`CustomerHeldKeyProvider`] (evaluation / pilot path).
//! - **`--wrapped-keys-dir`** — AWS KMS-wrapped seeds (`solum crypto wrap-seed`),
//!   feature `aws-kms`; CustomerHeld custody with `provider=aws-kms`.
//! - **`--ephemeral`** — [`EphemeralTestKeyProvider`] only with
//!   `SOLUM_ALLOW_EPHEMERAL=1` and a profile that allows `ephemeral_test`
//!   (e.g. `dev-local.toml`). Pilot profiles refuse EphemeralTest at startup.
//!
//! Envelope KMS unwraps seeds into process memory (ZeroizeOnDrop) — not an HSM/TEE.
//! See `docs/customer/SIDECAR-INTEGRATION.md`.
//!
//! [`SidecarKeys`] is a concrete enum so axum `State` stays sized (no `dyn`
//! provider). Ephemeral encrypt may call [`SharedEphemeralKeys::generate_test_keypair`]
//! on first use of a `key_ref`; CustomerHeld never auto-generates.
//!
//! # Access control
//!
//! 1. **Sidecar gate** — shared secret header (`X-Solum-Sidecar-Token`),
//!    constant-time compare. Fail → 401, no `Deployment` call.
//! 2. **Capabilities** — **org-IAM is required** on every profile except
//!    `dev-local` (`auth.allow_client_asserted_capabilities`). Body
//!    `capability[]` is not an authorization source on pilot profiles.
//!    Org-IAM: Bearer JWT (issuer + audience required) verified via JWKS;
//!    OIDC groups mapped to `CAP_*`. Stale JWKS refresh failure is **fail-closed**.
//! 3. **Consent + object binding** — Track B reads must name the consented
//!    subject *and* the resource / EHR / AQL must belong to that subject.
//!
//! # Track B CDR (H3.0, opt-in)
//!
//! When `--ehrbase-url` / `SOLUM_EHRBASE_URL` is set, `/v1/cdr/*` routes front
//! EHRbase and emit `cdr.*` audit events on successful writes. Without a URL,
//! those routes return 503 Track B disabled. FHIR / subject-link / dead-letter
//! JSONL is Crypt4GH-encrypted at rest.
//!
//! # TLS
//!
//! Plaintext HTTP is allowed only on loopback. Non-loopback bind is refused.
//! Terminate TLS at a reverse proxy in front of `127.0.0.1`. The sidecar is
//! not a TLS terminator.

#![forbid(unsafe_code)]

mod bind;
mod fhir_store;
mod listen;
mod store_crypto;
mod subject_link;

pub use fhir_store::{fhir_type_allowed, FhirStore, StoredFhirResource, ALLOWED_FHIR_TYPES};
pub use listen::{plaintext_http_env_allowed, validate_listen_bind};
pub use subject_link::{SubjectLink, SubjectLinkStore};

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use solum_auth_verify::{JwksVerifier, VerifyConfig};
use solum_core::audit::{events as audit_events, FileAuditStore};
use solum_core::crypto::{
    Crypt4ghKeyProvider, Crypt4ghKeys, CustomerHeldKeyProvider, EncryptedField,
    EphemeralTestKeyProvider, KeyCustody, KeyRef,
};
use solum_core::profiles::TransferMechanism;
use solum_core::{
    apply_runtime_env_overrides, example_eu_runtime, query_consent_status,
    require_operator_region_attestation, Deployment, SolumActor, SolumError,
};
use solum_identity::OrgCapMapping;
use solum_openehr::{
    aql_binds_subject, OpenEhrAdapter, OpenEhrError, PINNED_TEMPLATE_ID, PINNED_TEMPLATE_OPT,
};
use subtle::ConstantTimeEq;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

pub use solum_core::crypto::{KeypairFile, CUSTOMER_HELD_KEY_NOTE, EPHEMERAL_KEY_WARNING};

/// GET identity headers (capabilities must not travel in the query string).
pub const ACTOR_HEADER: &str = "x-solum-actor";
pub const CAPABILITY_HEADER: &str = "x-solum-capability";
pub const SUBJECT_HEADER: &str = "x-solum-subject";
pub const PURPOSE_HEADER: &str = "x-solum-purpose";

/// Honesty note for AWS KMS envelope path (feature `aws-kms` / `--wrapped-keys-dir`).
pub const AWS_KMS_KEY_NOTE: &str = "\
Using AWS KMS-wrapped Crypt4GH seeds from --wrapped-keys-dir (CustomerHeld custody; provider=aws-kms).
Seeds are unwrapped into process memory (ZeroizeOnDrop) — envelope encryption, not an HSM/TEE.
Requires build --features aws-kms and AWS credentials/region.";

/// Response / header name carrying ephemeral warning on crypto routes.
pub const EPHEMERAL_WARNING_HEADER: &str = "x-solum-ephemeral-keys";

/// Shared-secret header for the sidecar access gate (not GTM-1).
pub const SIDECAR_TOKEN_HEADER: &str = "x-solum-sidecar-token";

/// Shareable handle to a single [`EphemeralTestKeyProvider`].
#[derive(Clone)]
pub struct SharedEphemeralKeys(Arc<Mutex<EphemeralTestKeyProvider>>);

impl SharedEphemeralKeys {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(EphemeralTestKeyProvider::new())))
    }

    fn generate_test_keypair(&self, key_ref: KeyRef) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.0
            .lock()
            .map_err(|_| "ephemeral key provider lock poisoned".to_string())?
            .generate_test_keypair(key_ref)
            .map_err(|e| e.to_string())
    }

    /// Whether this session already holds a keypair for `key_ref`.
    fn key_exists(&self, key_ref: &KeyRef) -> Result<bool, String> {
        Ok(self
            .0
            .lock()
            .map_err(|_| "ephemeral key provider lock poisoned".to_string())?
            .recipient_pubkey(key_ref)
            .is_ok())
    }
}

impl Crypt4ghKeyProvider for SharedEphemeralKeys {
    fn recipient_pubkey(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<u8>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("ephemeral key lock poisoned".into())
            })?
            .recipient_pubkey(key_ref)
    }

    fn private_keys(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<Crypt4ghKeys>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("ephemeral key lock poisoned".into())
            })?
            .private_keys(key_ref)
    }
}

/// Shareable handle to a single [`CustomerHeldKeyProvider`].
#[derive(Clone)]
pub struct SharedCustomerHeldKeys(Arc<Mutex<CustomerHeldKeyProvider>>);

impl SharedCustomerHeldKeys {
    fn new(inner: CustomerHeldKeyProvider) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}

impl Crypt4ghKeyProvider for SharedCustomerHeldKeys {
    fn recipient_pubkey(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<u8>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("customer-held key lock poisoned".into())
            })?
            .recipient_pubkey(key_ref)
    }

    fn private_keys(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<Crypt4ghKeys>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("customer-held key lock poisoned".into())
            })?
            .private_keys(key_ref)
    }
}

/// Concrete key provider for axum `State` — CustomerHeld, AWS KMS, or gated ephemeral.
#[derive(Clone)]
pub enum SidecarKeys {
    Ephemeral(SharedEphemeralKeys),
    CustomerHeld(SharedCustomerHeldKeys),
    #[cfg(feature = "aws-kms")]
    AwsKms(SharedAwsKmsKeys),
}

#[cfg(feature = "aws-kms")]
#[derive(Clone)]
pub struct SharedAwsKmsKeys(Arc<Mutex<solum_core::crypto::aws_kms::AwsKmsKeyProvider>>);

#[cfg(feature = "aws-kms")]
impl SharedAwsKmsKeys {
    fn new(inner: solum_core::crypto::aws_kms::AwsKmsKeyProvider) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}

#[cfg(feature = "aws-kms")]
impl Crypt4ghKeyProvider for SharedAwsKmsKeys {
    fn recipient_pubkey(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<u8>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("aws-kms key lock poisoned".into())
            })?
            .recipient_pubkey(key_ref)
    }

    fn private_keys(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<Crypt4ghKeys>, solum_core::crypto::CryptoError> {
        self.0
            .lock()
            .map_err(|_| {
                solum_core::crypto::CryptoError::Provider("aws-kms key lock poisoned".into())
            })?
            .private_keys(key_ref)
    }
}

impl Crypt4ghKeyProvider for SidecarKeys {
    fn recipient_pubkey(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<u8>, solum_core::crypto::CryptoError> {
        match self {
            Self::Ephemeral(k) => k.recipient_pubkey(key_ref),
            Self::CustomerHeld(k) => k.recipient_pubkey(key_ref),
            #[cfg(feature = "aws-kms")]
            Self::AwsKms(k) => k.recipient_pubkey(key_ref),
        }
    }

    fn private_keys(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<Crypt4ghKeys>, solum_core::crypto::CryptoError> {
        match self {
            Self::Ephemeral(k) => k.private_keys(key_ref),
            Self::CustomerHeld(k) => k.private_keys(key_ref),
            #[cfg(feature = "aws-kms")]
            Self::AwsKms(k) => k.private_keys(key_ref),
        }
    }
}

/// Process-wide sidecar state: one Deployment over [`SidecarKeys`].
pub struct AppState {
    deployment: AsyncMutex<Deployment<SidecarKeys>>,
    keys: SidecarKeys,
    profile: PathBuf,
    audit_path: PathBuf,
    consent_path: PathBuf,
    /// Raw shared-secret bytes (from env); compared with [`subtle::ConstantTimeEq`].
    token: Vec<u8>,
    /// When set, mutating routes derive CAP_* from verified JWT groups (H2.2).
    org_iam: Option<OrgIamRuntime>,
    /// `dev-local` only: body `capability[]` may mint scopes. Pilot profiles false.
    allow_client_asserted: bool,
    /// Key used for FHIR / subject-link / dead-letter Crypt4GH envelopes.
    store_key_ref: KeyRef,
    encryption_categories: Vec<String>,
    /// Track B EHRbase base URL adapter (disabled when `cdr_url` is None).
    openehr: OpenEhrAdapter,
    /// Optional path to OPT XML; when unset, embedded pinned fixture is used.
    cdr_template_opt: Option<PathBuf>,
    fhir_store: AsyncMutex<FhirStore>,
    subject_link_store: AsyncMutex<SubjectLinkStore>,
    /// Dual-write dead-letter JSONL (H3.2 live webhook). Always set at startup.
    dual_write_dead_letter: PathBuf,
    /// Runtime used at [`Deployment::open`] (consent status must match custody).
    runtime: solum_core::profiles::RuntimeConfig,
}

/// Org-IAM runtime: JWKS verifier + group→CAP mapping.
pub struct OrgIamRuntime {
    mapping: OrgCapMapping,
    verifier: AsyncMutex<JwksVerifier>,
    jwks_url: Option<String>,
    verify_config: VerifyConfig,
    fetched_at: AsyncMutex<Option<std::time::Instant>>,
}

/// Startup configuration (CLI flags / env).
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub bind: SocketAddr,
    pub profile: PathBuf,
    pub audit: PathBuf,
    pub consent_store: PathBuf,
    pub token: String,
    /// Directory of `KeypairFile` JSON documents (`solum crypto keygen` layout).
    pub keys_dir: Option<PathBuf>,
    /// Dev-only ephemeral keys (conflicts with `keys_dir` / `wrapped_keys_dir`).
    pub ephemeral: bool,
    /// Directory of KMS-wrapped seed JSON (`solum crypto wrap-seed`). Requires `--features aws-kms`.
    pub wrapped_keys_dir: Option<PathBuf>,
    /// Org-IAM mapping TOML (`config/org-iam/*.toml`). Enables H2.2 mode when set.
    pub org_iam_config: Option<PathBuf>,
    /// JWKS URL (used when org-IAM is enabled). Env: `SOLUM_ORG_IAM_JWKS_URL`.
    pub jwks_url: Option<String>,
    /// Local JWKS JSON file (alternative to URL). Env: `SOLUM_ORG_IAM_JWKS_FILE`.
    pub jwks_file: Option<PathBuf>,
    /// Expected JWT issuer when org-IAM is enabled.
    pub oidc_issuer: Option<String>,
    /// Optional JWT audience (standalone OIDC).
    pub oidc_audience: Option<String>,
    /// EHRbase base URL including `/ehrbase` context (Track B). Env: `SOLUM_EHRBASE_URL`.
    pub ehrbase_url: Option<String>,
    /// Optional OPT file for template upload; default = embedded pinned fixture.
    pub cdr_template_opt: Option<PathBuf>,
    /// FHIR façade store (JSONL). Default: `<consent_store_dir>/fhir_store.jsonl`.
    pub fhir_store: Option<PathBuf>,
    /// Subject bridge store (JSONL). Default: `<consent_store_dir>/subject_links.jsonl`.
    pub subject_link_store: Option<PathBuf>,
    /// Dual-write dead-letter JSONL. Default: `<consent_store_dir>/dual_write_dead_letter.jsonl`.
    pub dual_write_dead_letter: Option<PathBuf>,
}

impl SidecarConfig {
    pub fn runtime_config(
        &self,
        custody: KeyCustody,
        provider: Option<&str>,
    ) -> solum_core::profiles::RuntimeConfig {
        let mut runtime = example_eu_runtime();
        apply_runtime_env_overrides(&mut runtime);
        runtime.key_management.provider =
            provider.map(|s| s.to_string()).or_else(|| match &custody {
                KeyCustody::CustomerHeld => Some("customer-held-file".into()),
                KeyCustody::EphemeralTest => Some("ephemeral-test".into()),
                KeyCustody::OperatorHeld => runtime.key_management.provider.clone(),
            });
        runtime.key_management.custody = custody;
        runtime
    }
}

fn ephemeral_env_allowed() -> bool {
    match std::env::var("SOLUM_ALLOW_EPHEMERAL") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

fn require_ephemeral_gate() -> Result<(), String> {
    if ephemeral_env_allowed() {
        return Ok(());
    }
    Err(
        "ephemeral crypto requires SOLUM_ALLOW_EPHEMERAL=1 (or true/yes). \
         Paid evaluations and pilots must use --keys-dir (CustomerHeld). \
         See docs/customer/SIDECAR-INTEGRATION.md / DEPLOYMENT-RUNBOOK.md §4."
            .into(),
    )
}

/// Load every regular file under `dir` as a [`KeypairFile`] and register it.
///
/// Fail-closed: unreadable or invalid JSON aborts with the file path; empty
/// directories and duplicate `key_ref` values are errors (no silent skip).
fn load_customer_held_from_dir(dir: &Path) -> Result<(CustomerHeldKeyProvider, KeyRef), String> {
    if !dir.is_dir() {
        return Err(format!("--keys-dir is not a directory: {}", dir.display()));
    }
    let mut provider = CustomerHeldKeyProvider::new();
    let mut loaded = 0usize;
    let mut seen_refs: Vec<String> = Vec::new();

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read --keys-dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read entry in {}: {e}", dir.display()))?;
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| format!("stat {}: {e}", path.display()))?;
        if !meta.is_file() {
            continue;
        }
        chmod_owner_rw(&path);
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read keypair {}: {e}", path.display()))?;
        let file: KeypairFile = serde_json::from_str(&raw).map_err(|e| {
            format!(
                "invalid keypair JSON {}: {e} (expected solum crypto keygen layout)",
                path.display()
            )
        })?;
        if seen_refs.iter().any(|r| r == &file.key_ref) {
            return Err(format!(
                "duplicate key_ref '{}' in --keys-dir (also in {})",
                file.key_ref,
                path.display()
            ));
        }
        let key_ref = KeyRef::new(file.key_ref.clone());
        provider
            .register_customer_keypair(key_ref, file.pubkey, file.privkey)
            .map_err(|e| format!("register keypair {}: {e}", path.display()))?;
        seen_refs.push(file.key_ref);
        loaded += 1;
    }

    if loaded == 0 {
        return Err(format!(
            "no keypair files found in --keys-dir {} (place solum crypto keygen JSON files here)",
            dir.display()
        ));
    }
    let store_key = provider
        .first_key_ref()
        .ok_or_else(|| "keys-dir loaded but no key_ref registered".to_string())?;
    Ok((provider, store_key))
}

fn chmod_owner_rw(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Build [`AppState`] (validates custody flags, profile, opens stores).
pub async fn build_state(config: &SidecarConfig) -> Result<Arc<AppState>, String> {
    if config.token.is_empty() {
        return Err("sidecar token must not be empty (set SOLUM_SIDECAR_TOKEN)".into());
    }

    let modes = [
        config.ephemeral,
        config.keys_dir.is_some(),
        config.wrapped_keys_dir.is_some(),
    ]
    .into_iter()
    .filter(|x| *x)
    .count();
    if modes > 1 {
        return Err("pass exactly one of --keys-dir, --wrapped-keys-dir, or --ephemeral".into());
    }
    if modes == 0 {
        return Err(
            "either --keys-dir, --wrapped-keys-dir (feature aws-kms), or --ephemeral required"
                .into(),
        );
    }

    let org_iam = load_org_iam(config).await?;

    let (keys, custody, provider, store_key_ref) = if config.ephemeral {
        require_ephemeral_gate()?;
        tracing::warn!("{EPHEMERAL_KEY_WARNING}");
        eprintln!("{EPHEMERAL_KEY_WARNING}");
        let eph = SharedEphemeralKeys::new();
        let store_key_ref = KeyRef::new("store/sidecar");
        eph.generate_test_keypair(store_key_ref.clone())?;
        (
            SidecarKeys::Ephemeral(eph),
            KeyCustody::EphemeralTest,
            Some("ephemeral-test"),
            store_key_ref,
        )
    } else if let Some(dir) = config.wrapped_keys_dir.as_ref() {
        #[cfg(feature = "aws-kms")]
        {
            use solum_core::crypto::aws_kms::{client_from_env, load_aws_kms_from_dir};
            tracing::info!("{AWS_KMS_KEY_NOTE}");
            eprintln!("{AWS_KMS_KEY_NOTE}");
            let client = client_from_env().map_err(|e| e.to_string())?;
            let provider = load_aws_kms_from_dir(&client, dir)
                .await
                .map_err(|e| e.to_string())?;
            let store_key_ref = provider
                .first_key_ref()
                .ok_or_else(|| "wrapped-keys-dir loaded but no key_ref registered".to_string())?;
            (
                SidecarKeys::AwsKms(SharedAwsKmsKeys::new(provider)),
                KeyCustody::CustomerHeld,
                Some("aws-kms"),
                store_key_ref,
            )
        }
        #[cfg(not(feature = "aws-kms"))]
        {
            let _ = dir;
            return Err(
                "--wrapped-keys-dir requires rebuilding solum-sidecar with --features aws-kms"
                    .into(),
            );
        }
    } else {
        let dir = config
            .keys_dir
            .as_ref()
            .expect("keys_dir checked non-None above");
        let (provider, store_key_ref) = load_customer_held_from_dir(dir)?;
        tracing::info!("{CUSTOMER_HELD_KEY_NOTE}");
        eprintln!("{CUSTOMER_HELD_KEY_NOTE}");
        (
            SidecarKeys::CustomerHeld(SharedCustomerHeldKeys::new(provider)),
            KeyCustody::CustomerHeld,
            Some("customer-held-file"),
            store_key_ref,
        )
    };

    let runtime = config.runtime_config(custody, provider);
    let deployment = Deployment::open(
        &config.profile,
        &runtime,
        &config.audit,
        &config.consent_store,
        keys.clone(),
    )
    .map_err(|e| e.to_string())?;

    require_operator_region_attestation(deployment.profile()).map_err(|e| e.to_string())?;

    let allow_client_asserted = deployment.profile().auth.allow_client_asserted_capabilities;
    if !allow_client_asserted && org_iam.is_none() {
        return Err(
            "this profile forbids client-asserted capability[]; start with --org-iam-config, \
             --jwks-url or --jwks-file, --oidc-issuer, and --oidc-audience \
             (only config/profiles/dev-local.toml allows capability[] for demos)"
                .into(),
        );
    }
    let encryption_categories = deployment
        .profile()
        .encryption
        .required_field_categories
        .clone();

    let fhir_path = config.fhir_store.clone().unwrap_or_else(|| {
        config
            .consent_store
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("fhir_store.jsonl")
    });
    let subject_path = config.subject_link_store.clone().unwrap_or_else(|| {
        config
            .consent_store
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("subject_links.jsonl")
    });
    let dual_write_dead_letter = config.dual_write_dead_letter.clone().unwrap_or_else(|| {
        config
            .consent_store
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("dual_write_dead_letter.jsonl")
    });
    if let Some(parent) = dual_write_dead_letter.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("dual-write dead-letter mkdir {}: {e}", parent.display()))?;
    }
    let fhir_store = FhirStore::open(
        &fhir_path,
        &keys,
        store_key_ref.clone(),
        encryption_categories.clone(),
    )?;
    let subject_link_store = SubjectLinkStore::open(
        &subject_path,
        &keys,
        store_key_ref.clone(),
        encryption_categories.clone(),
    )?;

    Ok(Arc::new(AppState {
        deployment: AsyncMutex::new(deployment),
        keys,
        profile: config.profile.clone(),
        audit_path: config.audit.clone(),
        consent_path: config.consent_store.clone(),
        token: config.token.as_bytes().to_vec(),
        org_iam,
        allow_client_asserted,
        store_key_ref,
        encryption_categories,
        openehr: match config
            .ehrbase_url
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(url) => OpenEhrAdapter::with_cdr_url(url),
            None => OpenEhrAdapter::new(),
        },
        cdr_template_opt: config.cdr_template_opt.clone(),
        fhir_store: AsyncMutex::new(fhir_store),
        subject_link_store: AsyncMutex::new(subject_link_store),
        dual_write_dead_letter,
        runtime,
    }))
}

async fn load_org_iam(config: &SidecarConfig) -> Result<Option<OrgIamRuntime>, String> {
    let Some(path) = config.org_iam_config.as_ref() else {
        return Ok(None);
    };
    let mapping = OrgCapMapping::load_from_path(path)?;
    let issuer = config.oidc_issuer.clone().ok_or_else(|| {
        "org-IAM requires --oidc-issuer (and --oidc-audience); Ferrum-passport tokens without aud are not accepted".to_string()
    })?;
    let audience = config.oidc_audience.clone().ok_or_else(|| {
        "org-IAM requires --oidc-audience together with --oidc-issuer".to_string()
    })?;
    let verify_config = VerifyConfig::for_standalone_oidc(issuer, audience);

    let verifier = if let Some(file) = config.jwks_file.as_ref() {
        let json = std::fs::read_to_string(file)
            .map_err(|e| format!("failed to read JWKS file {}: {e}", file.display()))?;
        JwksVerifier::from_jwks_json(&json, verify_config.clone()).map_err(|e| e.to_string())?
    } else if let Some(url) = config.jwks_url.as_ref() {
        JwksVerifier::from_url(url, verify_config.clone())
            .await
            .map_err(|e| e.to_string())?
    } else {
        return Err(
            "org-IAM requires --jwks-url or --jwks-file when --org-iam-config is set".into(),
        );
    };

    tracing::info!(
        claim_path = %mapping.claim_path,
        entries = mapping.entries.len(),
        "org-IAM enabled (H2.2): Bearer JWT groups → CAP_*"
    );
    Ok(Some(OrgIamRuntime {
        mapping,
        verifier: AsyncMutex::new(verifier),
        jwks_url: config.jwks_url.clone(),
        verify_config,
        fetched_at: AsyncMutex::new(Some(std::time::Instant::now())),
    }))
}

/// Axum router with auth middleware and `/v1/*` routes.
pub fn app_router(state: Arc<AppState>) -> Router {
    let authed = Router::new()
        .route("/v1/consent/grant", post(consent_grant))
        .route("/v1/consent/revoke", post(consent_revoke))
        .route("/v1/consent/status", get(consent_status))
        .route("/v1/crypto/encrypt", post(crypto_encrypt))
        .route("/v1/crypto/decrypt", post(crypto_decrypt))
        .route("/v1/audit/export", get(audit_export))
        .route("/v1/audit/verify", get(audit_verify))
        .route("/v1/cdr/template", post(cdr_upload_template))
        .route("/v1/cdr/ehr", post(cdr_create_ehr))
        .route(
            "/v1/cdr/ehr/:ehr_id/composition",
            post(cdr_commit_composition),
        )
        .route(
            "/v1/cdr/ehr/:ehr_id/composition/:composition_uid",
            get(cdr_get_composition),
        )
        .route("/v1/cdr/aql", post(cdr_aql))
        .route("/v1/transfer/check", post(transfer_check))
        .route("/v1/cdr/subject-link", post(subject_link_upsert))
        .route(
            "/v1/cdr/subject-link/:solum_subject_id",
            get(subject_link_get),
        )
        .route("/v1/fhir/:resource_type", post(fhir_create))
        .route("/v1/fhir/:resource_type/:id", get(fhir_get))
        .route("/v1/migrate/dual-write", post(migrate_dual_write))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            sidecar_token_middleware,
        ));

    Router::new()
        .merge(authed)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Same construction as CLI `cli_actor` in `solum-core` `main.rs`.
pub fn sidecar_actor(subject_id: String, capabilities: Vec<String>) -> SolumActor {
    SolumActor::standalone(subject_id, capabilities)
}

/// Resolve the actor for a mutating request (org-IAM or client capability[]).
async fn resolve_mutating_actor(
    state: &AppState,
    headers: &HeaderMap,
    body_actor: String,
    body_capability: Vec<String>,
) -> Result<SolumActor, Box<Response>> {
    let Some(org) = state.org_iam.as_ref() else {
        if !state.allow_client_asserted {
            return Err(Box::new(
                (
                    StatusCode::FORBIDDEN,
                    Json(ErrorBody {
                        error: "forbidden".into(),
                        message: "client-asserted capability[] is disabled for this profile; use org-IAM Bearer JWT".into(),
                    }),
                )
                    .into_response(),
            ));
        }
        return Ok(sidecar_actor(body_actor, body_capability));
    };

    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(token) = bearer else {
        return Err(Box::new(
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "unauthorized".into(),
                    message: "org-IAM requires Authorization: Bearer <jwt>".into(),
                }),
            )
                .into_response(),
        ));
    };

    const JWKS_TTL: std::time::Duration = std::time::Duration::from_secs(3600);
    if let Some(url) = org.jwks_url.as_ref() {
        let stale = {
            let fetched = org.fetched_at.lock().await;
            fetched.map(|t| t.elapsed() >= JWKS_TTL).unwrap_or(true)
        };
        if stale {
            match JwksVerifier::from_url(url, org.verify_config.clone()).await {
                Ok(fresh) => {
                    *org.verifier.lock().await = fresh;
                    *org.fetched_at.lock().await = Some(std::time::Instant::now());
                }
                Err(e) => {
                    return Err(Box::new(
                        (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(ErrorBody {
                                error: "jwks_refresh_failed".into(),
                                message: format!(
                                    "JWKS refresh failed; refusing to authenticate with stale keys: {e}"
                                ),
                            }),
                        )
                            .into_response(),
                    ));
                }
            }
        }
    }

    let verified = match org.verifier.lock().await.verify(token) {
        Ok(v) => v,
        Err(e) => {
            return Err(Box::new(
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorBody {
                        error: "unauthorized".into(),
                        message: format!("org-IAM JWT verify failed: {e}"),
                    }),
                )
                    .into_response(),
            ));
        }
    };

    let claim_vals = verified.claim_values(&org.mapping.claim_path);
    let scopes = org.mapping.resolve_capabilities(&claim_vals);
    let _ = body_capability;
    let actor = SolumActor {
        subject_id: verified.subject,
        display: if body_actor.is_empty() {
            None
        } else {
            Some(body_actor)
        },
        source: verified.actor_source,
        scopes,
    };
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        if let Some(iss) = verified.issuer.as_ref() {
            details.insert("issuer".into(), serde_json::Value::String(iss.clone()));
        }
        if let Err(e) = dep.record_identity_authenticated_as(&actor, details) {
            return Err(Box::new(map_solum_err(e)));
        }
    }
    Ok(actor)
}

async fn resolve_query_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SolumActor, Box<Response>> {
    let actor_id = header_nonempty(headers, ACTOR_HEADER).unwrap_or_default();
    let caps = header_nonempty(headers, CAPABILITY_HEADER)
        .map(|s| parse_capability_csv(&s))
        .unwrap_or_default();
    resolve_mutating_actor(state, headers, actor_id, caps).await
}

fn require_cap(actor: &SolumActor, capability: &str) -> Result<(), Box<Response>> {
    solum_identity::require_capability(actor, capability)
        .map_err(|e| Box::new(map_solum_err(e.into())))
}

fn object_not_bound(message: impl Into<String>) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            error: "object_not_bound".into(),
            message: message.into(),
        }),
    )
        .into_response()
}

async fn sidecar_token_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let provided = request
        .headers()
        .get(SIDECAR_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.as_bytes());

    let ok = match provided {
        Some(bytes) if bytes.len() == state.token.len() => {
            bool::from(bytes.ct_eq(state.token.as_slice()))
        }
        // Length mismatch: still touch the expected token so the failure path
        // is not a pure early-return oracle for length alone.
        Some(bytes) => {
            let _ = bytes.ct_eq(state.token.as_slice());
            false
        }
        None => {
            let _ = state.token.as_slice().ct_eq(state.token.as_slice());
            false
        }
    };

    if !ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: "unauthorized".into(),
                message: "missing or invalid X-Solum-Sidecar-Token".into(),
            }),
        )
            .into_response();
    }
    next.run(request).await
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

fn map_solum_err(err: SolumError) -> Response {
    match err {
        SolumError::Authorization(_) | SolumError::ConsentDenied { .. } => (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "forbidden".into(),
                message: err.to_string(),
            }),
        )
            .into_response(),
        SolumError::Audit(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "internal".into(),
                message: err.to_string(),
            }),
        )
            .into_response(),
        other => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: other.to_string(),
            }),
        )
            .into_response(),
    }
}

fn ephemeral_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str("not-for-production-ephemeral-test-keys") {
        headers.insert(EPHEMERAL_WARNING_HEADER, v);
    }
    headers
}

fn crypto_response_meta(keys: &SidecarKeys) -> (HeaderMap, &'static str) {
    match keys {
        SidecarKeys::Ephemeral(_) => (ephemeral_headers(), EPHEMERAL_KEY_WARNING),
        SidecarKeys::CustomerHeld(_) => (HeaderMap::new(), CUSTOMER_HELD_KEY_NOTE),
        #[cfg(feature = "aws-kms")]
        SidecarKeys::AwsKms(_) => (HeaderMap::new(), AWS_KMS_KEY_NOTE),
    }
}

/// Ephemeral-only: ensure a session keypair exists for `key_ref` (generate once).
/// CustomerHeld: no-op — unknown refs fail later inside `encrypt_field_as`.
fn ensure_ephemeral_key_for_encrypt(
    keys: &SidecarKeys,
    key_ref: &KeyRef,
) -> Result<(), Box<Response>> {
    match keys {
        SidecarKeys::CustomerHeld(_) => Ok(()),
        #[cfg(feature = "aws-kms")]
        SidecarKeys::AwsKms(_) => Ok(()),
        SidecarKeys::Ephemeral(ephemeral) => {
            // Reuse existing session keypair; only generate on first use.
            // (generate_test_keypair would silently overwrite HashMap entries.)
            match ephemeral.key_exists(key_ref) {
                Ok(true) => Ok(()),
                Ok(false) => ephemeral
                    .generate_test_keypair(key_ref.clone())
                    .map(|_| ())
                    .map_err(|e| {
                        Box::new(
                            (
                                StatusCode::BAD_REQUEST,
                                Json(ErrorBody {
                                    error: "bad_request".into(),
                                    message: e,
                                }),
                            )
                                .into_response(),
                        )
                    }),
                Err(e) => Err(Box::new(
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorBody {
                            error: "internal".into(),
                            message: e,
                        }),
                    )
                        .into_response(),
                )),
            }
        }
    }
}

// --- Request / response types (CLI-parameter parity) ---

#[derive(Debug, Deserialize)]
pub struct ConsentGrantRequest {
    pub subject: String,
    pub purpose: String,
    pub actor: String,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub capability: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConsentRevokeRequest {
    pub subject: String,
    pub purpose: String,
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConsentStatusQuery {
    pub subject: String,
    pub purpose: String,
}

#[derive(Debug, Serialize)]
struct ConsentStatusResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct EncryptRequest {
    pub category: String,
    /// Data subject — must have an active consent grant covering `category`.
    pub subject: String,
    /// Consent purpose paired with `subject`.
    pub purpose: String,
    pub key_ref: String,
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    /// Base64-encoded plaintext (HTTP stand-in for CLI `--in` file bytes).
    pub plaintext_base64: String,
}

#[derive(Debug, Serialize)]
struct EncryptResponse {
    field: EncryptedField,
    warning: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct DecryptRequest {
    /// Data subject — must have an active consent grant covering the field category.
    pub subject: String,
    /// Consent purpose paired with `subject`.
    pub purpose: String,
    pub key_ref: String,
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    pub field: EncryptedField,
}

#[derive(Debug, Serialize)]
struct DecryptResponse {
    plaintext_base64: String,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct AuditVerifyResponse {
    status: String,
}

async fn consent_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ConsentGrantRequest>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = state.deployment.lock().await;
    match deployment.grant_consent_as(&body.subject, &body.purpose, body.scope, &actor) {
        Ok(record) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn consent_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ConsentRevokeRequest>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = state.deployment.lock().await;
    match deployment.revoke_consent_as(&body.subject, &body.purpose, &actor) {
        Ok(record) => (StatusCode::OK, Json(record)).into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn consent_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ConsentStatusQuery>,
) -> Response {
    let actor = match resolve_query_actor(&state, &headers).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    if let Err(e) = require_cap(&actor, solum_identity::CAP_CONSENT_READ) {
        return *e;
    }
    match query_consent_status(
        &state.profile,
        &state.runtime,
        &state.consent_path,
        &q.subject,
        &q.purpose,
    ) {
        Ok(status) => (
            StatusCode::OK,
            Json(ConsentStatusResponse {
                status: status.to_string(),
            }),
        )
            .into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn crypto_encrypt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EncryptRequest>,
) -> Response {
    let plaintext = match base64::engine::general_purpose::STANDARD.decode(&body.plaintext_base64) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "bad_request".into(),
                    message: format!("plaintext_base64: {e}"),
                }),
            )
                .into_response();
        }
    };
    let key_ref = KeyRef::new(body.key_ref);
    if let Err(resp) = ensure_ephemeral_key_for_encrypt(&state.keys, &key_ref) {
        return *resp;
    }
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = state.deployment.lock().await;
    let (headers, warning) = crypto_response_meta(&state.keys);
    match deployment.encrypt_field_as(
        &body.category,
        &plaintext,
        &key_ref,
        &actor,
        &body.subject,
        &body.purpose,
    ) {
        Ok(field) => (
            StatusCode::OK,
            headers,
            Json(EncryptResponse { field, warning }),
        )
            .into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn crypto_decrypt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DecryptRequest>,
) -> Response {
    let key_ref = KeyRef::new(body.key_ref);
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = state.deployment.lock().await;
    let (headers, warning) = crypto_response_meta(&state.keys);
    match deployment.decrypt_field_as(&body.field, &key_ref, &actor, &body.subject, &body.purpose) {
        Ok(plaintext) => (
            StatusCode::OK,
            headers,
            Json(DecryptResponse {
                plaintext_base64: base64::engine::general_purpose::STANDARD.encode(plaintext),
                warning,
            }),
        )
            .into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn audit_export(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let actor = match resolve_query_actor(&state, &headers).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    if let Err(e) = require_cap(&actor, solum_identity::CAP_AUDIT_EXPORT) {
        return *e;
    }
    match FileAuditStore::open(&state.audit_path) {
        Ok(store) => match store.export_helios_json() {
            Ok(json) => {
                let mut dep = state.deployment.lock().await;
                if let Err(e) = dep.record_data_export_as(&actor, serde_json::Map::new()) {
                    return map_solum_err(e);
                }
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json")],
                    json,
                )
                    .into_response()
            }
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "bad_request".into(),
                    message: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn audit_verify(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let actor = match resolve_query_actor(&state, &headers).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    if let Err(e) = require_cap(&actor, solum_identity::CAP_AUDIT_VERIFY) {
        return *e;
    }
    match FileAuditStore::open(&state.audit_path) {
        Ok(store) => match store.verify_chain() {
            Ok(()) => (
                StatusCode::OK,
                Json(AuditVerifyResponse {
                    status: "ok".into(),
                }),
            )
                .into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "chain_broken".into(),
                    message: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: e.to_string(),
            }),
        )
            .into_response(),
    }
}

// --- Track B CDR façade (H3.0) ---

#[derive(Debug, Deserialize)]
pub struct CdrActorBody {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    /// When true (default for commit), fetch EHRbase canonical example for the template.
    #[serde(default)]
    pub use_example: Option<bool>,
    /// Optional FLAT composition JSON (used when `use_example` is false).
    #[serde(default)]
    pub composition: Option<serde_json::Value>,
    /// Template id; defaults to [`PINNED_TEMPLATE_ID`].
    #[serde(default)]
    pub template_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CdrReadQuery {
    pub actor: String,
    /// Comma-separated capability strings (GET cannot send JSON arrays cleanly).
    #[serde(default)]
    pub capability: String,
}

#[derive(Debug, Serialize)]
struct CdrEhrResponse {
    ehr_id: String,
}

#[derive(Debug, Serialize)]
struct CdrCompositionResponse {
    ehr_id: String,
    composition_uid: String,
    template_id: String,
}

#[derive(Debug, Serialize)]
struct CdrTemplateResponse {
    template_id: String,
    status: String,
}

fn map_openehr_err(err: OpenEhrError) -> Response {
    match err {
        OpenEhrError::TrackBDisabled => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "track_b_disabled".into(),
                message: err.to_string(),
            }),
        )
            .into_response(),
        OpenEhrError::AqlRejected => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "aql_rejected".into(),
                message: err.to_string(),
            }),
        )
            .into_response(),
        OpenEhrError::Status { status, body } => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: "ehrbase_error".into(),
                message: format!("EHRbase HTTP {status}: {body}"),
            }),
        )
            .into_response(),
        other => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: "ehrbase_error".into(),
                message: other.to_string(),
            }),
        )
            .into_response(),
    }
}

fn parse_capability_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn header_nonempty(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn missing_header(name: &str) -> Box<Response> {
    Box::new(
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: format!("missing header {name}"),
            }),
        )
            .into_response(),
    )
}

fn require_subject_purpose(
    subject: Option<&str>,
    purpose: Option<&str>,
) -> Result<(String, String), Box<Response>> {
    let subject = subject
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Box::new(
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody {
                        error: "bad_request".into(),
                        message: "subject is required for this operation".into(),
                    }),
                )
                    .into_response(),
            )
        })?;
    let purpose = purpose
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Box::new(
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody {
                        error: "bad_request".into(),
                        message: "purpose is required for this operation".into(),
                    }),
                )
                    .into_response(),
            )
        })?;
    Ok((subject.to_string(), purpose.to_string()))
}

async fn actor_from_get_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(SolumActor, String, String), Box<Response>> {
    let actor_id =
        header_nonempty(headers, ACTOR_HEADER).ok_or_else(|| missing_header(ACTOR_HEADER))?;
    let caps = header_nonempty(headers, CAPABILITY_HEADER)
        .map(|s| parse_capability_csv(&s))
        .unwrap_or_default();
    let subject =
        header_nonempty(headers, SUBJECT_HEADER).ok_or_else(|| missing_header(SUBJECT_HEADER))?;
    let purpose =
        header_nonempty(headers, PURPOSE_HEADER).ok_or_else(|| missing_header(PURPOSE_HEADER))?;
    let actor = resolve_mutating_actor(state, headers, actor_id, caps).await?;
    Ok((actor, subject, purpose))
}

fn require_fhir_bound_to_subject(
    resource_type: &str,
    resource: &serde_json::Value,
    subject: &str,
) -> Result<(), Box<Response>> {
    let id = resource.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if crate::bind::fhir_resource_belongs_to_subject(resource_type, id, resource, subject) {
        Ok(())
    } else {
        Err(Box::new(object_not_bound(format!(
            "{resource_type}/{id} is not bound to consented subject '{subject}'"
        ))))
    }
}

async fn require_ehr_bound_to_subject(
    state: &AppState,
    subject: &str,
    ehr_id: &str,
) -> Result<(), Response> {
    let store = state.subject_link_store.lock().await;
    let bound = match store.get(subject) {
        Ok(v) => v.and_then(|l| l.ehr_id),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: e,
                }),
            )
                .into_response());
        }
    };
    if bound.as_deref() == Some(ehr_id) {
        Ok(())
    } else {
        Err(object_not_bound(format!(
            "EHR '{ehr_id}' is not bound to consented subject '{subject}'"
        )))
    }
}

async fn upsert_subject_ehr_link(
    state: &AppState,
    actor: &SolumActor,
    subject: &str,
    ehr_id: &str,
) -> Result<(), String> {
    let mut store = state.subject_link_store.lock().await;
    let mut link = store.get(subject)?.unwrap_or(SubjectLink {
        solum_subject_id: subject.to_string(),
        ferrum_drs_id: None,
        phenopacket_id: None,
        ehr_id: None,
    });
    link.ehr_id = Some(ehr_id.to_string());
    store.upsert(&state.keys, &link)?;
    drop(store);
    let mut dep = state.deployment.lock().await;
    let mut details = serde_json::Map::new();
    details.insert(
        "solum_subject_id".into(),
        serde_json::Value::String(subject.to_string()),
    );
    details.insert(
        "ehr_id".into(),
        serde_json::Value::String(ehr_id.to_string()),
    );
    details.insert(
        "source".into(),
        serde_json::Value::String("cdr.ehr.created".into()),
    );
    dep.record_cdr_event_as(actor, audit_events::CDR_SUBJECT_LINK_UPSERTED, details)
        .map_err(|e| e.to_string())
}

async fn authorize_cdr_write_consented(
    state: &AppState,
    actor: &SolumActor,
    subject: &str,
    purpose: &str,
    operation: &str,
) -> Result<(), Response> {
    let mut dep = state.deployment.lock().await;
    dep.authorize_cdr_write_as(actor).map_err(map_solum_err)?;
    dep.require_consent_as(actor, subject, purpose, operation)
        .map_err(map_solum_err)
}

async fn authorize_cdr_read_consented(
    state: &AppState,
    actor: &SolumActor,
    subject: &str,
    purpose: &str,
    operation: &str,
) -> Result<(), Response> {
    let mut dep = state.deployment.lock().await;
    dep.authorize_cdr_read_as(actor).map_err(map_solum_err)?;
    dep.require_consent_as(actor, subject, purpose, operation)
        .map_err(map_solum_err)
}

fn load_template_opt(state: &AppState) -> Result<String, Box<Response>> {
    if let Some(path) = state.cdr_template_opt.as_ref() {
        return fs::read_to_string(path).map_err(|e| {
            Box::new(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: "internal".into(),
                        message: format!("failed to read CDR template OPT {}: {e}", path.display()),
                    }),
                )
                    .into_response(),
            )
        });
    }
    Ok(PINNED_TEMPLATE_OPT.to_string())
}

async fn cdr_upload_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CdrActorBody>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    {
        let mut dep = state.deployment.lock().await;
        if let Err(e) = dep.authorize_cdr_write_as(&actor) {
            return map_solum_err(e);
        }
    }
    let client = match state.openehr.client() {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    let opt = match load_template_opt(&state) {
        Ok(s) => s,
        Err(r) => return *r,
    };
    if let Err(e) = client.upload_template_opt(&opt).await {
        return map_openehr_err(e);
    }
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "template_id".into(),
            serde_json::Value::String(PINNED_TEMPLATE_ID.into()),
        );
        if let Err(e) =
            dep.record_cdr_event_as(&actor, audit_events::CDR_TEMPLATE_UPLOADED, details)
        {
            return map_solum_err(e);
        }
    }
    (
        StatusCode::OK,
        Json(CdrTemplateResponse {
            template_id: PINNED_TEMPLATE_ID.into(),
            status: "ok".into(),
        }),
    )
        .into_response()
}

async fn cdr_create_ehr(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CdrActorBody>,
) -> Response {
    let client = match state.openehr.client() {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let (subject, purpose) =
        match require_subject_purpose(body.subject.as_deref(), body.purpose.as_deref()) {
            Ok(v) => v,
            Err(r) => return *r,
        };
    if let Err(e) =
        authorize_cdr_write_consented(&state, &actor, &subject, &purpose, "cdr_create_ehr").await
    {
        return e;
    }
    let ehr_id = match client.create_ehr().await {
        Ok(id) => id,
        Err(e) => return map_openehr_err(e),
    };
    if let Err(e) = upsert_subject_ehr_link(&state, &actor, &subject, &ehr_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "internal".into(),
                message: e,
            }),
        )
            .into_response();
    }
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert("ehr_id".into(), serde_json::Value::String(ehr_id.clone()));
        if let Err(e) = dep.record_cdr_event_as(&actor, audit_events::CDR_EHR_CREATED, details) {
            return map_solum_err(e);
        }
    }
    (StatusCode::CREATED, Json(CdrEhrResponse { ehr_id })).into_response()
}

async fn cdr_commit_composition(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(ehr_id): AxumPath<String>,
    Json(body): Json<CdrActorBody>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let (subject, purpose) =
        match require_subject_purpose(body.subject.as_deref(), body.purpose.as_deref()) {
            Ok(v) => v,
            Err(r) => return *r,
        };
    if let Err(e) =
        authorize_cdr_write_consented(&state, &actor, &subject, &purpose, "cdr_commit_composition")
            .await
    {
        return e;
    }
    if let Err(resp) = require_ehr_bound_to_subject(&state, &subject, &ehr_id).await {
        return resp;
    }
    let client = match state.openehr.client() {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    let template_id = body
        .template_id
        .unwrap_or_else(|| PINNED_TEMPLATE_ID.to_string());
    let use_example = body.use_example.unwrap_or(false);
    let composition = if use_example {
        match client.example_composition(&template_id).await {
            Ok(v) => v,
            Err(e) => return map_openehr_err(e),
        }
    } else {
        match body.composition {
            Some(v) => v,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody {
                        error: "bad_request".into(),
                        message: "composition JSON required when use_example=false".into(),
                    }),
                )
                    .into_response();
            }
        }
    };
    let commit = match client.commit_composition(&ehr_id, &composition).await {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "ehr_id".into(),
            serde_json::Value::String(commit.ehr_id.clone()),
        );
        details.insert(
            "composition_uid".into(),
            serde_json::Value::String(commit.composition_uid.clone()),
        );
        details.insert(
            "template_id".into(),
            serde_json::Value::String(commit.template_id.clone()),
        );
        if let Err(e) =
            dep.record_cdr_event_as(&actor, audit_events::CDR_COMPOSITION_COMMITTED, details)
        {
            return map_solum_err(e);
        }
    }
    (
        StatusCode::CREATED,
        Json(CdrCompositionResponse {
            ehr_id: commit.ehr_id,
            composition_uid: commit.composition_uid,
            template_id: commit.template_id,
        }),
    )
        .into_response()
}

async fn cdr_get_composition(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((ehr_id, composition_uid)): AxumPath<(String, String)>,
) -> Response {
    let (actor, subject, purpose) = match actor_from_get_headers(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    if let Err(e) =
        authorize_cdr_read_consented(&state, &actor, &subject, &purpose, "cdr_get_composition")
            .await
    {
        return e;
    }
    if let Err(resp) = require_ehr_bound_to_subject(&state, &subject, &ehr_id).await {
        return resp;
    }
    let client = match state.openehr.client() {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    match client.get_composition(&ehr_id, &composition_uid).await {
        Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        Err(e) => map_openehr_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct AqlRequest {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    pub q: String,
}

async fn cdr_aql(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AqlRequest>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let (subject, purpose) =
        match require_subject_purpose(body.subject.as_deref(), body.purpose.as_deref()) {
            Ok(v) => v,
            Err(r) => return *r,
        };
    if let Err(e) =
        authorize_cdr_read_consented(&state, &actor, &subject, &purpose, "cdr_aql").await
    {
        return e;
    }
    if !aql_binds_subject(&body.q, &subject) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: format!(
                    "AQL must quote the consented subject '{subject}' (fail-closed object bind)"
                ),
            }),
        )
            .into_response();
    }
    let client = match state.openehr.client() {
        Ok(c) => c,
        Err(e) => return map_openehr_err(e),
    };
    let result = match client.execute_aql(&body.q).await {
        Ok(v) => v,
        Err(e) => return map_openehr_err(e),
    };
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "aql_len".into(),
            serde_json::Value::Number(body.q.len().into()),
        );
        if let Err(e) = dep.record_cdr_event_as(&actor, audit_events::CDR_AQL_EXECUTED, details) {
            return map_solum_err(e);
        }
    }
    (StatusCode::OK, Json(result)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct FhirWriteBody {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    /// FHIR resource JSON (must include resourceType matching path, or Bundle).
    pub resource: serde_json::Value,
    /// When Track B is enabled, also commit pinned openEHR composition and link ids.
    /// Default false until OPT mapping is honest (not a silent Patient→Observation rewrite).
    #[serde(default)]
    pub link_cdr: Option<bool>,
}

async fn fhir_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(resource_type): AxumPath<String>,
    Json(body): Json<FhirWriteBody>,
) -> Response {
    if !fhir_type_allowed(&resource_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: format!(
                    "resourceType '{resource_type}' not in H3.1 allowlist {:?}",
                    ALLOWED_FHIR_TYPES
                ),
            }),
        )
            .into_response();
    }
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let (subject, purpose) =
        match require_subject_purpose(body.subject.as_deref(), body.purpose.as_deref()) {
            Ok(v) => v,
            Err(r) => return *r,
        };
    if resource_type == "Patient" {
        let rid = body
            .resource
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if rid.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "bad_request".into(),
                    message:
                        "Patient.id is required (fail-closed; sidecar does not invent subject ids)"
                            .into(),
                }),
            )
                .into_response();
        }
        if rid != subject {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "bad_request".into(),
                    message: format!("subject '{subject}' must match Patient.id '{rid}'"),
                }),
            )
                .into_response();
        }
    }
    if let Err(resp) = require_fhir_bound_to_subject(&resource_type, &body.resource, &subject) {
        return *resp;
    }
    if let Err(e) =
        authorize_cdr_write_consented(&state, &actor, &subject, &purpose, "fhir_create").await
    {
        return e;
    }

    let link_cdr = body.link_cdr.unwrap_or(false);
    match persist_fhir_resource(&state, &actor, &resource_type, body.resource, link_cdr).await {
        Ok(stored) => (StatusCode::CREATED, Json(stored.resource)).into_response(),
        Err(msg)
            if msg.starts_with("body resourceType")
                || msg.starts_with("Patient.id")
                || msg.starts_with("link_cdr") =>
        {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "bad_request".into(),
                    message: msg,
                }),
            )
                .into_response()
        }
        Err(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "internal".into(),
                message: msg,
            }),
        )
            .into_response(),
    }
}

struct PersistFhirOk {
    resource: serde_json::Value,
    ehr_id: Option<String>,
    composition_uid: Option<String>,
}

/// Shared FHIR façade write used by `/v1/fhir/*` and `/v1/migrate/dual-write`.
async fn persist_fhir_resource(
    state: &AppState,
    actor: &solum_core::SolumActor,
    resource_type: &str,
    mut resource: serde_json::Value,
    link_cdr: bool,
) -> Result<PersistFhirOk, String> {
    let declared = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !declared.is_empty() && declared != resource_type {
        return Err(format!(
            "body resourceType '{declared}' != path '{resource_type}'"
        ));
    }
    if declared.is_empty() {
        if let Some(obj) = resource.as_object_mut() {
            obj.insert(
                "resourceType".into(),
                serde_json::Value::String(resource_type.to_string()),
            );
        }
    }
    let id = match resource.get("id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ if resource_type == "Patient" => {
            return Err(
                "Patient.id is required (fail-closed; sidecar does not invent subject ids)".into(),
            );
        }
        _ => Uuid::new_v4().to_string(),
    };
    if let Some(obj) = resource.as_object_mut() {
        obj.insert("id".into(), serde_json::Value::String(id.clone()));
    }

    if link_cdr {
        return Err(
            "link_cdr is refused: the sidecar does not commit EHRbase example compositions as patient data. \
             Persist FHIR with link_cdr=false and commit a real composition via POST /v1/cdr/.../composition with use_example=false"
                .into(),
        );
    }

    let stored = StoredFhirResource {
        resource_type: resource_type.to_string(),
        id: id.clone(),
        resource: resource.clone(),
        ehr_id: None,
        composition_uid: None,
    };
    {
        let mut store = state.fhir_store.lock().await;
        store.upsert(&state.keys, &stored)?;
    }

    // Patient → subject bridge (H3.3): same id string partners should use as Ferrum solum_subject.
    if resource_type == "Patient" {
        let link = SubjectLink {
            solum_subject_id: id.clone(),
            ferrum_drs_id: None,
            phenopacket_id: None,
            ehr_id: None,
        };
        {
            let mut store = state.subject_link_store.lock().await;
            store.upsert(&state.keys, &link)?;
        }
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "solum_subject_id".into(),
            serde_json::Value::String(id.clone()),
        );
        details.insert(
            "source".into(),
            serde_json::Value::String("fhir.Patient".into()),
        );
        dep.record_cdr_event_as(actor, audit_events::CDR_SUBJECT_LINK_UPSERTED, details)
            .map_err(|e| e.to_string())?;
    }

    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "resource_type".into(),
            serde_json::Value::String(resource_type.to_string()),
        );
        details.insert("id".into(), serde_json::Value::String(id));
        dep.record_cdr_event_as(actor, audit_events::CDR_FHIR_CREATED, details)
            .map_err(|e| e.to_string())?;
        dep.record_data_receive_eehrxf_as(actor, serde_json::Map::new())
            .map_err(|e| e.to_string())?;
    }

    Ok(PersistFhirOk {
        resource,
        ehr_id: None,
        composition_uid: None,
    })
}

#[derive(Debug, Deserialize)]
pub struct DualWriteBody {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    /// FHIR resource JSON (must include resourceType).
    pub resource: serde_json::Value,
    /// Optional legacy system id / correlation for dead-letter triage.
    #[serde(default)]
    pub source: Option<String>,
    /// When true, refused: sidecar will not commit EHRbase example compositions as patient data.
    #[serde(default)]
    pub link_cdr: Option<bool>,
}

/// Live dual-write webhook: mirror FHIR into Solum façade (+ optional CDR).
/// On mirror failure → append dead-letter JSONL and return **202** (never silent drop).
async fn migrate_dual_write(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DualWriteBody>,
) -> Response {
    let actor =
        match resolve_mutating_actor(&state, &headers, body.actor.clone(), body.capability).await {
            Ok(a) => a,
            Err(r) => return *r,
        };
    let (subject, purpose) =
        match require_subject_purpose(body.subject.as_deref(), body.purpose.as_deref()) {
            Ok(v) => v,
            Err(r) => return *r,
        };
    if let Err(e) =
        authorize_cdr_write_consented(&state, &actor, &subject, &purpose, "dual_write").await
    {
        return e;
    }

    let resource_type = body
        .resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if resource_type.is_empty() || !fhir_type_allowed(&resource_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: format!(
                    "resourceType '{resource_type}' not in H3.1 allowlist {:?}",
                    ALLOWED_FHIR_TYPES
                ),
            }),
        )
            .into_response();
    }
    if let Err(resp) = require_fhir_bound_to_subject(&resource_type, &body.resource, &subject) {
        return *resp;
    }

    let link_cdr = body.link_cdr.unwrap_or(false);
    match persist_fhir_resource(
        &state,
        &actor,
        &resource_type,
        body.resource.clone(),
        link_cdr,
    )
    .await
    {
        Ok(stored) => {
            let mut dep = state.deployment.lock().await;
            let mut details = serde_json::Map::new();
            details.insert(
                "resource_type".into(),
                serde_json::Value::String(resource_type),
            );
            if let Some(s) = body.source {
                details.insert("source".into(), serde_json::Value::String(s));
            }
            if let Some(e) = stored.ehr_id {
                details.insert("ehr_id".into(), serde_json::Value::String(e));
            }
            if let Some(c) = stored.composition_uid {
                details.insert("composition_uid".into(), serde_json::Value::String(c));
            }
            if let Err(e) =
                dep.record_cdr_event_as(&actor, audit_events::CDR_DUAL_WRITE_OK, details)
            {
                return map_solum_err(e);
            }
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "dead_lettered": false,
                    "resource": stored.resource,
                })),
            )
                .into_response()
        }
        Err(reason) => {
            let row = solum_core::dead_letter_row(
                &reason,
                &serde_json::json!({
                    "source": body.source,
                    "resource": body.resource,
                }),
            );
            let envelope = match crate::store_crypto::encrypt_store_json(
                &state.keys,
                &state.encryption_categories,
                &state.store_key_ref,
                crate::store_crypto::FHIR_STORE_CATEGORY,
                &row,
            ) {
                Ok(f) => f,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorBody {
                            error: "internal".into(),
                            message: format!(
                                "dual-write failed and dead-letter encrypt failed: {e}"
                            ),
                        }),
                    )
                        .into_response();
                }
            };
            let envelope_val = match serde_json::to_value(&envelope) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorBody {
                            error: "internal".into(),
                            message: format!("dead-letter serialize: {e}"),
                        }),
                    )
                        .into_response();
                }
            };
            if let Err(e) =
                solum_core::append_dead_letter(&state.dual_write_dead_letter, &envelope_val)
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: "internal".into(),
                        message: format!("dual-write failed and dead-letter write failed: {e}"),
                    }),
                )
                    .into_response();
            }
            {
                let mut dep = state.deployment.lock().await;
                let mut details = serde_json::Map::new();
                details.insert("reason".into(), serde_json::Value::String(reason.clone()));
                details.insert(
                    "dead_letter".into(),
                    serde_json::Value::String(state.dual_write_dead_letter.display().to_string()),
                );
                if let Err(e) = dep.record_cdr_event_as(
                    &actor,
                    audit_events::CDR_DUAL_WRITE_DEAD_LETTERED,
                    details,
                ) {
                    return map_solum_err(e);
                }
            }
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "dead_lettered": true,
                    "reason": reason,
                    "dead_letter": state.dual_write_dead_letter.display().to_string(),
                })),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FhirReadQuery {
    pub actor: String,
    #[serde(default)]
    pub capability: String,
}

async fn fhir_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((resource_type, id)): AxumPath<(String, String)>,
) -> Response {
    if !fhir_type_allowed(&resource_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "bad_request".into(),
                message: format!("resourceType '{resource_type}' not allowed"),
            }),
        )
            .into_response();
    }
    let (actor, subject, purpose) = match actor_from_get_headers(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    if let Err(e) =
        authorize_cdr_read_consented(&state, &actor, &subject, &purpose, "fhir_get").await
    {
        return e;
    }
    let found = {
        let store = state.fhir_store.lock().await;
        match store.get(&resource_type, &id) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: "internal".into(),
                        message: e,
                    }),
                )
                    .into_response();
            }
        }
    };
    match found {
        Some(entry) => {
            if !crate::bind::fhir_resource_belongs_to_subject(
                &resource_type,
                &id,
                &entry.resource,
                &subject,
            ) {
                return object_not_bound(format!(
                    "{resource_type}/{id} is not bound to consented subject '{subject}'"
                ));
            }
            (StatusCode::OK, Json(entry.resource)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: "not_found".into(),
                message: format!("{resource_type}/{id} not found"),
            }),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SubjectLinkBody {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    pub solum_subject_id: String,
    #[serde(default)]
    pub ferrum_drs_id: Option<String>,
    #[serde(default)]
    pub phenopacket_id: Option<String>,
    #[serde(default)]
    pub ehr_id: Option<String>,
}

async fn subject_link_upsert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubjectLinkBody>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let purpose =
        match require_subject_purpose(Some(&body.solum_subject_id), body.purpose.as_deref()) {
            Ok((_, p)) => p,
            Err(r) => return *r,
        };
    if let Err(e) = authorize_cdr_write_consented(
        &state,
        &actor,
        &body.solum_subject_id,
        &purpose,
        "subject_link_upsert",
    )
    .await
    {
        return e;
    }
    let link = SubjectLink {
        solum_subject_id: body.solum_subject_id.clone(),
        ferrum_drs_id: body.ferrum_drs_id.clone(),
        phenopacket_id: body.phenopacket_id.clone(),
        ehr_id: body.ehr_id.clone(),
    };
    {
        let mut store = state.subject_link_store.lock().await;
        if let Err(e) = store.upsert(&state.keys, &link) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: e,
                }),
            )
                .into_response();
        }
    }
    {
        let mut dep = state.deployment.lock().await;
        let mut details = serde_json::Map::new();
        details.insert(
            "solum_subject_id".into(),
            serde_json::Value::String(body.solum_subject_id),
        );
        if let Err(e) =
            dep.record_cdr_event_as(&actor, audit_events::CDR_SUBJECT_LINK_UPSERTED, details)
        {
            return map_solum_err(e);
        }
    }
    (StatusCode::OK, Json(link)).into_response()
}

async fn subject_link_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(solum_subject_id): AxumPath<String>,
) -> Response {
    let (actor, subject, purpose) = match actor_from_get_headers(&state, &headers).await {
        Ok(v) => v,
        Err(r) => return *r,
    };
    if let Err(e) =
        authorize_cdr_read_consented(&state, &actor, &subject, &purpose, "subject_link_get").await
    {
        return e;
    }
    if solum_subject_id != subject {
        return object_not_bound(format!(
            "subject-link '{solum_subject_id}' is not the consented subject '{subject}'"
        ));
    }
    let found = {
        let store = state.subject_link_store.lock().await;
        match store.get(&solum_subject_id) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: "internal".into(),
                        message: e,
                    }),
                )
                    .into_response();
            }
        }
    };
    match found {
        Some(link) => (StatusCode::OK, Json(link)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: "not_found".into(),
                message: format!("subject link '{solum_subject_id}' not found"),
            }),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct TransferCheckBody {
    pub actor: String,
    #[serde(default)]
    pub capability: Vec<String>,
    pub mechanism: TransferMechanism,
    pub destination: String,
}

async fn transfer_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<TransferCheckBody>,
) -> Response {
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability).await {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut dep = state.deployment.lock().await;
    match dep.check_transfer_as(&body.mechanism, &body.destination, &actor) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response(),
        Err(e) => map_solum_err(e),
    }
}

/// Serve the router on `config.bind` until the process is stopped.
pub async fn serve(config: SidecarConfig) -> Result<(), String> {
    validate_listen_bind(config.bind, plaintext_http_env_allowed())?;
    let state = build_state(&config).await?;
    if !config.bind.ip().is_loopback() && !state.allow_client_asserted {
        return Err(
            "non-loopback HTTP is refused on pilot profiles. Bind 127.0.0.1 and terminate TLS \
             at a reverse proxy. SOLUM_ALLOW_PLAINTEXT_HTTP=1 is honoured only with dev-local."
                .into(),
        );
    }
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|e| format!("bind {}: {e}", config.bind))?;
    tracing::info!(%config.bind, "solum-sidecar listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}
