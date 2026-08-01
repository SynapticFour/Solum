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
//! # Key custody — Option A (not production)
//!
//! This crate fixes crypto to [`EphemeralTestKeyProvider`] only — no
//! `aws-kms` feature, no customer-held registration path. Keys live in process
//! memory for the lifetime of the sidecar. See [`EPHEMERAL_KEY_WARNING`] and
//! `docs/customer/SIDECAR-INTEGRATION.md`.
//!
//! `Deployment` owns its key provider privately, so encrypt must call
//! [`EphemeralTestKeyProvider::generate_test_keypair`] on the same map that
//! `Deployment` later reads. [`SharedEphemeralKeys`] is a thin
//! [`Crypt4ghKeyProvider`] handle over `Arc<Mutex<EphemeralTestKeyProvider>>`
//! so generate + encrypt/decrypt share one store. It is **not** a second
//! custody model.
//!
//! # Access control layers
//!
//! 1. **Sidecar gate** — shared secret header (`X-Solum-Sidecar-Token`),
//!    constant-time compare. Fail → 401, no `Deployment` call.
//! 2. **GTM-1** — `actor` + `capability[]` → [`SolumActor`] (same as CLI
//!    `cli_actor`), checked inside `*_as`. Fail → 403, no side effect.

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
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
use solum_core::audit::FileAuditStore;
use solum_core::crypto::{
    Crypt4ghKeyProvider, Crypt4ghKeys, EncryptedField, EphemeralTestKeyProvider, KeyRef,
};
use solum_core::{
    example_eu_runtime, query_consent_status, ActorSource, Deployment, SolumActor, SolumError,
};
use subtle::ConstantTimeEq;
use tower_http::trace::TraceLayer;

/// Same warning text as the CLI (`solum-core` binary) ephemeral path.
pub const EPHEMERAL_KEY_WARNING: &str = "\
⚠ Using EphemeralTestKeyProvider — keys are NOT persisted across runs
and are NOT suitable for real patient data. Production key custody
(CustomerHeld / HSM-backed) is not yet wired into the sidecar.
Keys exist only in the sidecar process memory for this run; restarting
the process loses them. Demo-only — not an HSM.";

/// Response / header name carrying [`EPHEMERAL_KEY_WARNING`] on crypto routes.
pub const EPHEMERAL_WARNING_HEADER: &str = "x-solum-ephemeral-keys";

/// Shared-secret header for the sidecar access gate (not GTM-1).
pub const SIDECAR_TOKEN_HEADER: &str = "x-solum-sidecar-token";

/// Shareable handle to a single [`EphemeralTestKeyProvider`] (Option A only).
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
    ///
    /// Uses [`Crypt4ghKeyProvider::recipient_pubkey`] (`Err` for unknown refs)
    /// — no change to `EphemeralTestKeyProvider` generate/overwrite behaviour.
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

/// Process-wide sidecar state: one Deployment over shared ephemeral keys.
pub struct AppState {
    deployment: Mutex<Deployment<SharedEphemeralKeys>>,
    keys: SharedEphemeralKeys,
    profile: PathBuf,
    audit_path: PathBuf,
    consent_path: PathBuf,
    /// Raw shared-secret bytes (from env); compared with [`subtle::ConstantTimeEq`].
    token: Vec<u8>,
}

/// Startup configuration (CLI flags / env).
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub bind: SocketAddr,
    pub profile: PathBuf,
    pub audit: PathBuf,
    pub consent_store: PathBuf,
    pub token: String,
}

impl SidecarConfig {
    pub fn runtime_config(&self) -> solum_core::profiles::RuntimeConfig {
        let mut runtime = example_eu_runtime();
        if let Ok(region) = std::env::var("SOLUM_STORAGE_REGION") {
            runtime.storage_region = region;
        }
        runtime
    }
}

/// Build [`AppState`] (validates profile, opens stores). Logs ephemeral warning.
pub fn build_state(config: &SidecarConfig) -> Result<Arc<AppState>, String> {
    tracing::warn!("{EPHEMERAL_KEY_WARNING}");
    eprintln!("{EPHEMERAL_KEY_WARNING}");

    if config.token.is_empty() {
        return Err("sidecar token must not be empty (set SOLUM_SIDECAR_TOKEN)".into());
    }

    let keys = SharedEphemeralKeys::new();
    let deployment = Deployment::open(
        &config.profile,
        &config.runtime_config(),
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
    Json(body): Json<ConsentGrantRequest>,
) -> Response {
    let actor = sidecar_actor(body.actor, body.capability);
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
    Json(body): Json<ConsentRevokeRequest>,
) -> Response {
    let actor = sidecar_actor(body.actor, body.capability);
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
    let mut runtime = example_eu_runtime();
    if let Ok(region) = std::env::var("SOLUM_STORAGE_REGION") {
        runtime.storage_region = region;
    }
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
    // Reuse existing session keypair for this key_ref; only generate on first use.
    // (generate_test_keypair would silently overwrite HashMap entries — see crypto crate.)
    match state.keys.key_exists(&key_ref) {
        Ok(true) => {}
        Ok(false) => {
            if let Err(e) = state.keys.generate_test_keypair(key_ref.clone()) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody {
                        error: "bad_request".into(),
                        message: e,
                    }),
                )
                    .into_response();
            }
        }
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
    let actor = sidecar_actor(body.actor, body.capability);
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
    match deployment.encrypt_field_as(&body.category, &plaintext, &key_ref, &actor) {
        Ok(field) => (
            StatusCode::OK,
            ephemeral_headers(),
            Json(EncryptResponse {
                field,
                warning: EPHEMERAL_KEY_WARNING,
            }),
        )
            .into_response(),
        Err(e) => map_solum_err(e),
    }
}

async fn crypto_decrypt(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DecryptRequest>,
) -> Response {
    let key_ref = KeyRef::new(body.key_ref);
    let actor = sidecar_actor(body.actor, body.capability);
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
    match deployment.decrypt_field_as(&body.field, &key_ref, &actor) {
        Ok(plaintext) => (
            StatusCode::OK,
            ephemeral_headers(),
            Json(DecryptResponse {
                plaintext_base64: base64::engine::general_purpose::STANDARD.encode(plaintext),
                warning: EPHEMERAL_KEY_WARNING,
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
    let state = build_state(&config)?;
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|e| format!("bind {}: {e}", config.bind))?;
    tracing::info!(%config.bind, "solum-sidecar listening (loopback default; demo keys only)");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}
