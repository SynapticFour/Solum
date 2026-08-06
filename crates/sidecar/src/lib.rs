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
//! - **`--ephemeral`** — [`EphemeralTestKeyProvider`] only with
//!   `SOLUM_ALLOW_EPHEMERAL=1` and a profile that allows `ephemeral_test`
//!   (e.g. `dev-local.toml`). Pilot profiles refuse EphemeralTest at startup.
//!
//! AWS KMS is **not** wired here (follow-on). See
//! `docs/customer/SIDECAR-INTEGRATION.md`.
//!
//! [`SidecarKeys`] is a concrete enum so axum `State` stays sized (no `dyn`
//! provider). Ephemeral encrypt may call [`SharedEphemeralKeys::generate_test_keypair`]
//! on first use of a `key_ref`; CustomerHeld never auto-generates.
//!
//! # Access control layers
//!
//! 1. **Sidecar gate** — shared secret header (`X-Solum-Sidecar-Token`),
//!    constant-time compare. Fail → 401, no `Deployment` call.
//! 2. **GTM-1 capabilities** — default: body `capability[]` → [`SolumActor`]
//!    (same as CLI). **H2.2 org-IAM mode:** Bearer JWT verified via JWKS;
//!    OIDC groups mapped to `CAP_*` from a TOML file; body `capability[]`
//!    ignored. Fail → 403, no side effect.

#![forbid(unsafe_code)]

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use solum_auth_verify::{JwksVerifier, VerifyConfig};
use solum_core::audit::FileAuditStore;
use solum_core::crypto::{
    Crypt4ghKeyProvider, Crypt4ghKeys, CustomerHeldKeyProvider, EncryptedField,
    EphemeralTestKeyProvider, KeyCustody, KeyRef,
};
use solum_core::{
    example_eu_runtime, query_consent_status, ActorSource, Deployment, SolumActor, SolumError,
};
use solum_identity::OrgCapMapping;
use subtle::ConstantTimeEq;
use tower_http::trace::TraceLayer;

/// Same warning text as the CLI (`solum-core` binary) ephemeral path.
pub const EPHEMERAL_KEY_WARNING: &str = "\
⚠ Using EphemeralTestKeyProvider — keys are NOT persisted across runs
and are NOT suitable for real patient data or paid evaluations.
Requires SOLUM_ALLOW_EPHEMERAL=1 and a profile that allows ephemeral_test
(e.g. config/profiles/dev-local.toml). Pilot profiles (eu-ehds, kenya-dpa)
refuse EphemeralTest custody at startup.
Keys exist only in the sidecar process memory for this run; restarting
the process loses them. Demo-only — not an HSM.";

/// Same honesty note as the CLI CustomerHeld `--keypair` path.
pub const CUSTOMER_HELD_KEY_NOTE: &str = "\
Using CustomerHeld key material from --keys-dir (operator-supplied files).
Solum does not mint these keys during encrypt; protect keypair files
as you would other secrets (0600 on Unix recommended).";

/// Response / header name carrying ephemeral warning on crypto routes.
pub const EPHEMERAL_WARNING_HEADER: &str = "x-solum-ephemeral-keys";

/// Shared-secret header for the sidecar access gate (not GTM-1).
pub const SIDECAR_TOKEN_HEADER: &str = "x-solum-sidecar-token";

/// Same JSON layout as CLI `KeypairFile` (`solum crypto keygen` output).
///
/// `pubkey` / `privkey` are raw byte arrays (serde JSON number arrays), matching
/// the CLI — not a divergent sidecar schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeypairFile {
    pub key_ref: String,
    pub pubkey: Vec<u8>,
    pub privkey: Vec<u8>,
}

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

/// Concrete key provider for axum `State` — CustomerHeld or gated ephemeral.
#[derive(Clone)]
pub enum SidecarKeys {
    Ephemeral(SharedEphemeralKeys),
    CustomerHeld(SharedCustomerHeldKeys),
}

impl Crypt4ghKeyProvider for SidecarKeys {
    fn recipient_pubkey(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<u8>, solum_core::crypto::CryptoError> {
        match self {
            Self::Ephemeral(k) => k.recipient_pubkey(key_ref),
            Self::CustomerHeld(k) => k.recipient_pubkey(key_ref),
        }
    }

    fn private_keys(
        &self,
        key_ref: &KeyRef,
    ) -> Result<Vec<Crypt4ghKeys>, solum_core::crypto::CryptoError> {
        match self {
            Self::Ephemeral(k) => k.private_keys(key_ref),
            Self::CustomerHeld(k) => k.private_keys(key_ref),
        }
    }
}

/// Process-wide sidecar state: one Deployment over [`SidecarKeys`].
pub struct AppState {
    deployment: Mutex<Deployment<SidecarKeys>>,
    keys: SidecarKeys,
    profile: PathBuf,
    audit_path: PathBuf,
    consent_path: PathBuf,
    /// Raw shared-secret bytes (from env); compared with [`subtle::ConstantTimeEq`].
    token: Vec<u8>,
    /// When set, mutating routes derive CAP_* from verified JWT groups (H2.2).
    org_iam: Option<OrgIamRuntime>,
}

/// Org-IAM runtime: JWKS verifier + group→CAP mapping.
#[derive(Clone)]
pub struct OrgIamRuntime {
    mapping: OrgCapMapping,
    verifier: JwksVerifier,
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
    /// Dev-only ephemeral keys (conflicts with `keys_dir` at the CLI layer).
    pub ephemeral: bool,
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
}

impl SidecarConfig {
    pub fn runtime_config(&self, custody: KeyCustody) -> solum_core::profiles::RuntimeConfig {
        let mut runtime = example_eu_runtime();
        if let Ok(region) = std::env::var("SOLUM_STORAGE_REGION") {
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
fn load_customer_held_from_dir(dir: &Path) -> Result<CustomerHeldKeyProvider, String> {
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
    Ok(provider)
}

/// Build [`AppState`] (validates custody flags, profile, opens stores).
pub async fn build_state(config: &SidecarConfig) -> Result<Arc<AppState>, String> {
    if config.token.is_empty() {
        return Err("sidecar token must not be empty (set SOLUM_SIDECAR_TOKEN)".into());
    }

    if config.ephemeral && config.keys_dir.is_some() {
        return Err("pass either --keys-dir or --ephemeral, not both".into());
    }
    if !config.ephemeral && config.keys_dir.is_none() {
        return Err("either --keys-dir or --ephemeral required".into());
    }

    let org_iam = load_org_iam(config).await?;

    let (keys, custody) = if config.ephemeral {
        require_ephemeral_gate()?;
        tracing::warn!("{EPHEMERAL_KEY_WARNING}");
        eprintln!("{EPHEMERAL_KEY_WARNING}");
        (
            SidecarKeys::Ephemeral(SharedEphemeralKeys::new()),
            KeyCustody::EphemeralTest,
        )
    } else {
        let dir = config
            .keys_dir
            .as_ref()
            .expect("keys_dir checked non-None above");
        let provider = load_customer_held_from_dir(dir)?;
        tracing::info!("{CUSTOMER_HELD_KEY_NOTE}");
        eprintln!("{CUSTOMER_HELD_KEY_NOTE}");
        (
            SidecarKeys::CustomerHeld(SharedCustomerHeldKeys::new(provider)),
            KeyCustody::CustomerHeld,
        )
    };

    let deployment = Deployment::open(
        &config.profile,
        &config.runtime_config(custody),
        &config.audit,
        &config.consent_store,
        keys.clone(),
    )
    .map_err(|e| e.to_string())?;

    Ok(Arc::new(AppState {
        deployment: Mutex::new(deployment),
        keys,
        profile: config.profile.clone(),
        audit_path: config.audit.clone(),
        consent_path: config.consent_store.clone(),
        token: config.token.as_bytes().to_vec(),
        org_iam,
    }))
}

async fn load_org_iam(config: &SidecarConfig) -> Result<Option<OrgIamRuntime>, String> {
    let Some(path) = config.org_iam_config.as_ref() else {
        return Ok(None);
    };
    let mapping = OrgCapMapping::load_from_path(path)?;
    let verify_config = if let Some(aud) = config.oidc_audience.as_ref() {
        let issuer = config
            .oidc_issuer
            .clone()
            .ok_or_else(|| "org-IAM with --oidc-audience requires --oidc-issuer".to_string())?;
        VerifyConfig::for_standalone_oidc(issuer, aud.clone())
    } else {
        let mut cfg = VerifyConfig::for_ferrum_passport();
        cfg.expected_issuer = config.oidc_issuer.clone();
        cfg
    };

    let verifier = if let Some(file) = config.jwks_file.as_ref() {
        let json = std::fs::read_to_string(file)
            .map_err(|e| format!("failed to read JWKS file {}: {e}", file.display()))?;
        JwksVerifier::from_jwks_json(&json, verify_config).map_err(|e| e.to_string())?
    } else if let Some(url) = config.jwks_url.as_ref() {
        JwksVerifier::from_url(url, verify_config)
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
    Ok(Some(OrgIamRuntime { mapping, verifier }))
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
    SolumActor {
        subject_id,
        display: None,
        source: ActorSource::LocalDev,
        scopes: capabilities,
    }
}

/// Resolve the actor for a mutating request (org-IAM or client capability[]).
fn resolve_mutating_actor(
    state: &AppState,
    headers: &HeaderMap,
    body_actor: String,
    body_capability: Vec<String>,
) -> Result<SolumActor, Box<Response>> {
    let Some(org) = state.org_iam.as_ref() else {
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

    let verified = match org.verifier.verify(token) {
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
    // body.capability intentionally ignored in org-IAM mode
    let _ = body_capability;
    Ok(SolumActor {
        subject_id: verified.subject,
        display: if body_actor.is_empty() {
            None
        } else {
            Some(body_actor)
        },
        source: verified.actor_source,
        scopes,
    })
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
        SolumError::Authorization(e) => (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "forbidden".into(),
                message: e.to_string(),
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
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = match state.deployment.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: "deployment lock poisoned".into(),
                }),
            )
                .into_response();
        }
    };
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
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = match state.deployment.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: "deployment lock poisoned".into(),
                }),
            )
                .into_response();
        }
    };
    match deployment.revoke_consent_as(&body.subject, &body.purpose, &actor) {
        Ok(record) => (StatusCode::OK, Json(record)).into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn consent_status(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConsentStatusQuery>,
) -> Response {
    // Consent status does not touch Crypt4GH keys — CustomerHeld runtime matches
    // pilot profiles (same as CLI consent status).
    let runtime = {
        let mut runtime = example_eu_runtime();
        if let Ok(region) = std::env::var("SOLUM_STORAGE_REGION") {
            runtime.storage_region = region;
        }
        runtime
    };
    match query_consent_status(
        &state.profile,
        &runtime,
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
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = match state.deployment.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: "deployment lock poisoned".into(),
                }),
            )
                .into_response();
        }
    };
    let (headers, warning) = crypto_response_meta(&state.keys);
    match deployment.encrypt_field_as(&body.category, &plaintext, &key_ref, &actor) {
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
    let actor = match resolve_mutating_actor(&state, &headers, body.actor, body.capability) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let mut deployment = match state.deployment.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal".into(),
                    message: "deployment lock poisoned".into(),
                }),
            )
                .into_response();
        }
    };
    let (headers, warning) = crypto_response_meta(&state.keys);
    match deployment.decrypt_field_as(&body.field, &key_ref, &actor) {
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

async fn audit_export(State(state): State<Arc<AppState>>) -> Response {
    // Read-only open of the same path Deployment appends to (fsync'd per append).
    match FileAuditStore::open(&state.audit_path) {
        Ok(store) => match store.export_helios_json() {
            Ok(json) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                json,
            )
                .into_response(),
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

async fn audit_verify(State(state): State<Arc<AppState>>) -> Response {
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

/// Serve the router on `config.bind` until the process is stopped.
pub async fn serve(config: SidecarConfig) -> Result<(), String> {
    let state = build_state(&config).await?;
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|e| format!("bind {}: {e}", config.bind))?;
    tracing::info!(%config.bind, "solum-sidecar listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}
