//! HTTP integration tests for `solum-sidecar` (axum + reqwest against a free port).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use serde_json::Value;
use solum_core::crypto::generate_operator_keypair;
use solum_sidecar::{
    app_router, build_state, validate_listen_bind, KeypairFile, SidecarConfig,
    CUSTOMER_HELD_KEY_NOTE, EPHEMERAL_WARNING_HEADER, SIDECAR_TOKEN_HEADER,
};
use tempfile::tempdir;

const TEST_OIDC_ISSUER: &str = "https://idp.test/oidc";
const TEST_OIDC_AUD: &str = "solum-api";

fn org_iam_mapping() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/org-iam/pilot-groups.toml")
}
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn eu_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
}

fn dev_local_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/dev-local.toml")
}

fn write_keypair_file(dir: &Path, key_ref: &str) -> PathBuf {
    let (pubkey, privkey) = generate_operator_keypair().expect("generate_operator_keypair");
    let path = dir.join(format!("{}.json", key_ref.replace('/', "_")));
    let file = KeypairFile {
        key_ref: key_ref.to_string(),
        pubkey,
        privkey,
    };
    std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();
    path
}

/// Ephemeral sidecar: `--ephemeral` + env gate + `dev-local` profile.
#[allow(clippy::await_holding_lock)] // env gate must stay set for the whole build_state
async fn spawn_ephemeral_sidecar(token: &str) -> (SocketAddr, tempfile::TempDir) {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_ALLOW_EPHEMERAL", "1");
    let dir = tempdir().unwrap();
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: dev_local_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
        keys_dir: None,
        ephemeral: true,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let state = build_state(&config).await.expect("build_state ephemeral");
    // SOLUM_ALLOW_EPHEMERAL is read only synchronously inside build_state()
    // (require_ephemeral_gate()); app_router() and the TCP bind do not touch
    // the env var. Release before the first .await so we do not hold
    // std::sync::MutexGuard across await (clippy::await_holding_lock).
    drop(_guard);
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, dir)
}

/// CustomerHeld sidecar: `--keys-dir` with a pre-registered keypair + org-IAM (eu-ehds).
#[allow(clippy::await_holding_lock)]
async fn spawn_customer_held_sidecar(
    token: &str,
    key_ref: &str,
) -> (SocketAddr, tempfile::TempDir, String) {
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, key_ref);
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_STORAGE_REGION", "EU");
    let (jwks_src, jwt, jwks_hold) = mint_rsa_jwks_and_token(&[
        "solum-consent-ops",
        "solum-crypto-ops",
        "solum-cdr-ops",
        "solum-audit-ops",
    ]);
    let jwks = dir.path().join("jwks.json");
    std::fs::copy(&jwks_src, &jwks).unwrap();
    drop(jwks_hold);
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: Some(org_iam_mapping()),
        jwks_url: None,
        jwks_file: Some(jwks),
        oidc_issuer: Some(TEST_OIDC_ISSUER.into()),
        oidc_audience: Some(TEST_OIDC_AUD.into()),
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let state = build_state(&config)
        .await
        .expect("build_state customer-held");
    drop(_guard);
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, dir, jwt)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn grant_care_provision_http(addr: &str, token: &str) {
    grant_care_provision_for(addr, token, "patient/42", None).await;
}

async fn grant_care_provision_for(addr: &str, token: &str, subject: &str, bearer: Option<&str>) {
    let mut req = client()
        .post(format!("http://{addr}/v1/consent/grant"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": subject,
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }));
    if let Some(jwt) = bearer {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let res = req.send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    assert!(
        status == 200 || status == 201,
        "grant failed: status={status} body={body}"
    );
}

#[tokio::test]
async fn grant_with_capability_and_secret_created() {
    let token = "test-secret-grant-ok";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();
    assert_eq!(status, 201, "body={body}");
}

#[tokio::test]
async fn grant_without_secret_is_unauthorized() {
    let token = "test-secret-no-header";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);

    let status_url =
        format!("http://{addr}/v1/consent/status?subject=patient%2F42&purpose=care_provision");
    let status = client()
        .get(&status_url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/7")
        .header("X-Solum-Capability", "solum:consent:read")
        .send()
        .await
        .unwrap();
    assert_eq!(status.status(), 200);
    let body: Value = status.json().await.unwrap();
    assert_eq!(body["status"], "unknown");
    drop(dir);
}

#[tokio::test]
async fn grant_wrong_secret_is_unauthorized() {
    let token = "test-secret-correct";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, "wrong-token-value")
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn grant_missing_capability_is_forbidden_no_side_effect() {
    let token = "test-secret-cap-deny";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": [],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();
    assert_eq!(status, 403, "body={body}");

    let status_url =
        format!("http://{addr}/v1/consent/status?subject=patient%2F42&purpose=care_provision");
    let status = client()
        .get(&status_url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/7")
        .header("X-Solum-Capability", "solum:consent:read")
        .send()
        .await
        .unwrap();
    let body: Value = status.json().await.unwrap();
    assert_eq!(body["status"], "unknown");
}

#[tokio::test]
async fn encrypt_decrypt_round_trip_over_http() {
    let token = "test-secret-crypto";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_http(&addr.to_string(), token).await;
    let plain = b"patient-summary-demo";
    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": "ephemeral/sidecar-1",
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(plain)
        }))
        .send()
        .await
        .unwrap();
    assert!(enc.headers().get(EPHEMERAL_WARNING_HEADER).is_some());
    let enc_status = enc.status();
    let enc_body: Value = enc.json().await.unwrap();
    assert_eq!(enc_status, 200, "body={enc_body}");
    assert!(enc_body["warning"]
        .as_str()
        .unwrap()
        .contains("EphemeralTestKeyProvider"));
    let field = enc_body["field"].clone();

    let dec = client()
        .post(format!("http://{addr}/v1/crypto/decrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": "ephemeral/sidecar-1",
            "actor": "practitioner/7",
            "capability": ["solum:crypto:decrypt"],
            "field": field
        }))
        .send()
        .await
        .unwrap();
    assert!(dec.headers().get(EPHEMERAL_WARNING_HEADER).is_some());
    let dec_status = dec.status();
    let dec_body: Value = dec.json().await.unwrap();
    assert_eq!(dec_status, 200, "body={dec_body}");
    let out = base64::engine::general_purpose::STANDARD
        .decode(dec_body["plaintext_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(out, plain);
}

#[tokio::test]
async fn encrypt_same_key_ref_twice_both_ciphertexts_decrypt() {
    let token = "test-secret-key-reuse";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_http(&addr.to_string(), token).await;
    let key_ref = "ephemeral/reuse-1";
    let plain_a = b"first-plaintext-block";
    let plain_b = b"second-plaintext-block";

    let enc_a = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(plain_a)
        }))
        .send()
        .await
        .unwrap();
    let status_a = enc_a.status();
    let body_a: Value = enc_a.json().await.unwrap();
    assert_eq!(status_a, 200, "body={body_a}");
    let field_a = body_a["field"].clone();

    let enc_b = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(plain_b)
        }))
        .send()
        .await
        .unwrap();
    let status_b = enc_b.status();
    let body_b: Value = enc_b.json().await.unwrap();
    assert_eq!(status_b, 200, "body={body_b}");
    let field_b = body_b["field"].clone();

    let dec_a = client()
        .post(format!("http://{addr}/v1/crypto/decrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:decrypt"],
            "field": field_a
        }))
        .send()
        .await
        .unwrap();
    let dec_a_status = dec_a.status();
    let dec_a_body: Value = dec_a.json().await.unwrap();
    assert_eq!(dec_a_status, 200, "body={dec_a_body}");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(dec_a_body["plaintext_base64"].as_str().unwrap())
            .unwrap(),
        plain_a
    );

    let dec_b = client()
        .post(format!("http://{addr}/v1/crypto/decrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:decrypt"],
            "field": field_b
        }))
        .send()
        .await
        .unwrap();
    let dec_b_status = dec_b.status();
    let dec_b_body: Value = dec_b.json().await.unwrap();
    assert_eq!(dec_b_status, 200, "body={dec_b_body}");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(dec_b_body["plaintext_base64"].as_str().unwrap())
            .unwrap(),
        plain_b
    );
}

#[tokio::test]
async fn audit_verify_ok_after_operations() {
    let token = "test-secret-audit";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_http(&addr.to_string(), token).await;

    let grant = client()
        .post(format!("http://{addr}/v1/consent/grant"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(grant.status(), 201);

    let plain = b"audit-trail-demo";
    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": "ephemeral/audit-1",
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(plain)
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(enc.status(), 200);

    let verify = client()
        .get(format!("http://{addr}/v1/audit/verify"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/7")
        .header("X-Solum-Capability", "solum:audit:verify")
        .send()
        .await
        .unwrap();
    assert_eq!(verify.status(), 200);
    let body: Value = verify.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let export = client()
        .get(format!("http://{addr}/v1/audit/export"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/7")
        .header("X-Solum-Capability", "solum:audit:export")
        .send()
        .await
        .unwrap();
    assert_eq!(export.status(), 200);
    let exported: Value = export.json().await.unwrap();
    assert!(exported["record_count"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn customer_held_encrypt_decrypt_round_trip() {
    let token = "test-secret-ch-roundtrip";
    let key_ref = "customer/sidecar-1";
    let (addr, _dir, jwt) = spawn_customer_held_sidecar(token, key_ref).await;
    grant_care_provision_for(&addr.to_string(), token, "patient/42", Some(&jwt)).await;
    let plain = b"customer-held-plaintext";

    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(plain)
        }))
        .send()
        .await
        .unwrap();
    assert!(enc.headers().get(EPHEMERAL_WARNING_HEADER).is_none());
    let enc_status = enc.status();
    let enc_body: Value = enc.json().await.unwrap();
    assert_eq!(enc_status, 200, "body={enc_body}");
    assert_eq!(
        enc_body["warning"].as_str().unwrap(),
        CUSTOMER_HELD_KEY_NOTE
    );
    let field = enc_body["field"].clone();

    let dec = client()
        .post(format!("http://{addr}/v1/crypto/decrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": key_ref,
            "actor": "practitioner/7",
            "capability": ["solum:crypto:decrypt"],
            "field": field
        }))
        .send()
        .await
        .unwrap();
    let dec_status = dec.status();
    let dec_body: Value = dec.json().await.unwrap();
    assert_eq!(dec_status, 200, "body={dec_body}");
    let out = base64::engine::general_purpose::STANDARD
        .decode(dec_body["plaintext_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(out, plain);
}

#[tokio::test]
async fn customer_held_unknown_key_ref_does_not_auto_generate() {
    let token = "test-secret-ch-unknown";
    let (addr, _dir, jwt) = spawn_customer_held_sidecar(token, "customer/known-1").await;
    grant_care_provision_for(&addr.to_string(), token, "patient/42", Some(&jwt)).await;

    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "category": "patient_summary",
            "subject": "patient/42",
            "purpose": "care_provision",
            "key_ref": "customer/never-registered",
            "actor": "practitioner/7",
            "capability": ["solum:crypto:encrypt"],
            "plaintext_base64": base64::engine::general_purpose::STANDARD.encode(b"x")
        }))
        .send()
        .await
        .unwrap();
    let status = enc.status();
    let body: Value = enc.json().await.unwrap();
    assert_eq!(status, 400, "body={body}");
    let msg = body["message"].as_str().unwrap_or("").to_ascii_lowercase();
    assert!(
        msg.contains("unknown") || msg.contains("key") || msg.contains("never-registered"),
        "expected unknown-key error, got {body}"
    );
}

#[tokio::test]
async fn build_state_requires_keys_dir_or_ephemeral() {
    let dir = tempdir().unwrap();
    let err = match build_state(&SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "tok".into(),
        keys_dir: None,
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    })
    .await
    {
        Ok(_) => panic!("must require custody flag"),
        Err(e) => e,
    };
    assert!(
        err.contains("--keys-dir") && err.contains("--ephemeral"),
        "err={err}"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // env must stay unset for the whole build_state
async fn build_state_ephemeral_requires_allow_env() {
    let _guard = env_lock().lock().unwrap();
    std::env::remove_var("SOLUM_ALLOW_EPHEMERAL");
    let dir = tempdir().unwrap();
    let err = match build_state(&SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: dev_local_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "tok".into(),
        keys_dir: None,
        ephemeral: true,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    })
    .await
    {
        Ok(_) => panic!("must require SOLUM_ALLOW_EPHEMERAL"),
        Err(e) => e,
    };
    assert!(err.contains("SOLUM_ALLOW_EPHEMERAL"), "err={err}");
    // Restore for sibling ephemeral HTTP tests that may run after in the same process.
    std::env::set_var("SOLUM_ALLOW_EPHEMERAL", "1");
}

// --- H2.2 org-IAM ---

fn mint_rsa_jwks_and_token(groups: &[&str]) -> (PathBuf, String, tempfile::TempDir) {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    mint_rsa_jwks_and_token_claims(serde_json::json!({
        "sub": "practitioner/org-iam",
        "iss": TEST_OIDC_ISSUER,
        "aud": TEST_OIDC_AUD,
        "exp": t + 3600,
        "groups": groups,
    }))
}

fn mint_rsa_jwks_and_token_claims(
    claims: serde_json::Value,
) -> (PathBuf, String, tempfile::TempDir) {
    use base64::Engine;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use rand::rngs::OsRng;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    let dir = tempdir().unwrap();
    let mut rng = OsRng;
    let private = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let public = RsaPublicKey::from(&private);
    let pem = private.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
    let encoding = EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
    let n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
    let kid = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(public.n().to_bytes_be()));
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "kid": kid,
            "use": "sig",
            "alg": "RS256",
            "n": n,
            "e": e,
        }]
    });
    let jwks_path = dir.path().join("jwks.json");
    std::fs::write(&jwks_path, jwks.to_string()).unwrap();

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid);
    let token = encode(&header, &claims, &encoding).unwrap();
    (jwks_path, token, dir)
}

#[allow(clippy::await_holding_lock)]
async fn spawn_org_iam_sidecar(
    token: &str,
    jwks_file: PathBuf,
    mapping_path: PathBuf,
) -> (SocketAddr, tempfile::TempDir) {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_ALLOW_EPHEMERAL", "1");
    let dir = tempdir().unwrap();
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: dev_local_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
        keys_dir: None,
        ephemeral: true,
        wrapped_keys_dir: None,
        org_iam_config: Some(mapping_path),
        jwks_url: None,
        jwks_file: Some(jwks_file),
        oidc_issuer: Some(TEST_OIDC_ISSUER.into()),
        oidc_audience: Some(TEST_OIDC_AUD.into()),
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let state = build_state(&config).await.expect("build_state org-iam");
    drop(_guard);
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, dir)
}

#[tokio::test]
async fn org_iam_grant_with_mapped_group() {
    let mapping =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/org-iam/pilot-groups.toml");
    let (jwks, jwt, _keydir) = mint_rsa_jwks_and_token(&["solum-consent-ops"]);
    let token = "org-iam-secret";
    let (addr, dir) = spawn_org_iam_sidecar(token, jwks, mapping).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "ignored-for-caps",
            "capability": [],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201, "body={}", res.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(
        audit.contains("identity.authenticated"),
        "org-IAM success must emit identity.authenticated: {audit}"
    );
    assert!(
        audit.contains("standalone:practitioner/org-iam"),
        "consent/audit must bind the IdP sub, not a Ferrum Passport: {audit}"
    );
}

#[tokio::test]
async fn org_iam_keycloak_hospital_realm_roles() {
    let mapping = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/org-iam/keycloak-hospital.toml");
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let (jwks, jwt, _keydir) = mint_rsa_jwks_and_token_claims(serde_json::json!({
        "sub": "practitioner/org-iam",
        "iss": TEST_OIDC_ISSUER,
        "aud": TEST_OIDC_AUD,
        "exp": t + 3600,
        "realm_access": { "roles": ["solum-consent-ops"] },
    }));
    let token = "org-iam-keycloak-hospital";
    let (addr, dir) = spawn_org_iam_sidecar(token, jwks, mapping).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "ignored-for-caps",
            "capability": [],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201, "body={}", res.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(
        audit.contains("standalone:practitioner/org-iam"),
        "Keycloak hospital tokens bind the clinician sub, not a Ferrum Passport: {audit}"
    );
}

#[tokio::test]
async fn org_iam_rejects_capability_only_without_group() {
    let mapping =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/org-iam/pilot-groups.toml");
    let (jwks, jwt, _keydir) = mint_rsa_jwks_and_token(&["unrelated-group"]);
    let token = "org-iam-secret-deny";
    let (addr, _dir) = spawn_org_iam_sidecar(token, jwks, mapping).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "attacker",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403, "body={}", res.text().await.unwrap());
}

#[tokio::test]
async fn org_iam_requires_bearer() {
    let mapping =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/org-iam/pilot-groups.toml");
    let (jwks, _jwt, _keydir) = mint_rsa_jwks_and_token(&["solum-consent-ops"]);
    let token = "org-iam-secret-nobearer";
    let (addr, _dir) = spawn_org_iam_sidecar(token, jwks, mapping).await;
    let url = format!("http://{addr}/v1/consent/grant");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "subject": "patient/42",
            "purpose": "care_provision",
            "actor": "practitioner/7",
            "capability": ["solum:consent:grant"],
            "scope": ["patient_summary"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401, "body={}", res.text().await.unwrap());
}

/// Minimal EHRbase mock matching solum-openehr client paths.
async fn spawn_mock_ehrbase() -> SocketAddr {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::routing::{get, post};
    use axum::{Json, Router};

    async fn create_ehr() -> impl axum::response::IntoResponse {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::LOCATION,
            HeaderValue::from_static(
                "http://mock/ehrbase/rest/openehr/v1/ehr/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            ),
        );
        (StatusCode::CREATED, headers, Json(serde_json::json!({})))
    }
    async fn upload_template() -> StatusCode {
        StatusCode::CREATED
    }
    async fn example_flat() -> impl axum::response::IntoResponse {
        Json(serde_json::json!({
            "_type": "COMPOSITION",
            "name": { "value": "Minimal" },
            "archetype_details": {
                "template_id": { "value": "minimal_observation.en.v1" }
            }
        }))
    }
    async fn commit(
        axum::extract::Path(ehr_id): axum::extract::Path<String>,
    ) -> impl axum::response::IntoResponse {
        let mut headers = HeaderMap::new();
        let loc = format!(
            "http://mock/ehrbase/rest/openehr/v1/ehr/{ehr_id}/composition/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"
        );
        headers.insert(
            axum::http::header::LOCATION,
            HeaderValue::from_str(&loc).unwrap(),
        );
        (
            StatusCode::CREATED,
            headers,
            Json(serde_json::json!({
                "uid": { "value": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb" },
                "archetype_details": {
                    "template_id": { "value": "minimal_observation.en.v1" }
                }
            })),
        )
    }
    async fn get_comp(
        axum::extract::Path((_e, uid)): axum::extract::Path<(String, String)>,
    ) -> impl axum::response::IntoResponse {
        Json(serde_json::json!({ "uid": { "value": uid } }))
    }

    let app = Router::new()
        .route("/ehrbase/rest/openehr/v1/ehr", post(create_ehr))
        .route(
            "/ehrbase/rest/openehr/v1/definition/template/adl1.4",
            post(upload_template),
        )
        .route(
            "/ehrbase/rest/openehr/v1/definition/template/adl1.4/:id/example",
            get(example_flat),
        )
        .route(
            "/ehrbase/rest/openehr/v1/ehr/:ehr_id/composition",
            post(commit),
        )
        .route(
            "/ehrbase/rest/openehr/v1/ehr/:ehr_id/composition/:uid",
            get(get_comp),
        )
        .route(
            "/ehrbase/rest/openehr/v1/query/aql",
            post(|| async {
                axum::Json(serde_json::json!({
                    "meta": { "_type": "RESULTSET" },
                    "q": "SELECT c FROM EHR e CONTAINS COMPOSITION c",
                    "rows": []
                }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

#[allow(clippy::await_holding_lock)] // env gate must stay set for the whole build_state
async fn spawn_ephemeral_sidecar_with_ehrbase(
    token: &str,
    ehrbase_url: String,
) -> (SocketAddr, tempfile::TempDir) {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_ALLOW_EPHEMERAL", "1");
    let dir = tempdir().unwrap();
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: dev_local_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
        keys_dir: None,
        ephemeral: true,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: Some(ehrbase_url),
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let state = build_state(&config)
        .await
        .expect("build_state ephemeral+cdr");
    drop(_guard);
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, dir)
}

#[tokio::test]
async fn cdr_disabled_without_ehrbase_url() {
    let token = "cdr-disabled-token";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    let url = format!("http://{addr}/v1/cdr/ehr");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 503, "body={}", res.text().await.unwrap());
}

#[tokio::test]
async fn cdr_write_denied_without_capability() {
    let ehr = spawn_mock_ehrbase().await;
    let token = "cdr-deny-token";
    let (addr, dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, format!("http://{ehr}/ehrbase")).await;
    let url = format!("http://{addr}/v1/cdr/ehr");
    let res = client()
        .post(&url)
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": [],
            "subject": "patient/42",
            "purpose": "care_provision"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403, "body={}", res.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("access.denied"));
}

#[tokio::test]
async fn cdr_facade_write_read_and_audit() {
    let ehr = spawn_mock_ehrbase().await;
    let token = "cdr-ok-token";
    let (addr, dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, format!("http://{ehr}/ehrbase")).await;

    let tmpl = client()
        .post(format!("http://{addr}/v1/cdr/template"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(tmpl.status(), 200, "body={}", tmpl.text().await.unwrap());

    grant_care_provision_http(&addr.to_string(), token).await;

    let ehr_res = client()
        .post(format!("http://{addr}/v1/cdr/ehr"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "patient/42",
            "purpose": "care_provision"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        ehr_res.status(),
        201,
        "body={}",
        ehr_res.text().await.unwrap()
    );
    let ehr_body: Value = ehr_res.json().await.unwrap();
    let ehr_id = ehr_body["ehr_id"].as_str().unwrap();

    let comp_res = client()
        .post(format!("http://{addr}/v1/cdr/ehr/{ehr_id}/composition"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "patient/42",
            "purpose": "care_provision",
            "use_example": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        comp_res.status(),
        201,
        "body={}",
        comp_res.text().await.unwrap()
    );
    let comp: Value = comp_res.json().await.unwrap();
    let uid = comp["composition_uid"].as_str().unwrap();

    let get_res = client()
        .get(format!(
            "http://{addr}/v1/cdr/ehr/{ehr_id}/composition/{uid}"
        ))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/h3")
        .header("X-Solum-Capability", "solum:cdr:read")
        .header("X-Solum-Subject", "patient/42")
        .header("X-Solum-Purpose", "care_provision")
        .send()
        .await
        .unwrap();
    assert_eq!(
        get_res.status(),
        200,
        "body={}",
        get_res.text().await.unwrap()
    );

    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.template.uploaded"), "audit={audit}");
    assert!(audit.contains("cdr.ehr.created"), "audit={audit}");
    assert!(audit.contains("cdr.composition.committed"), "audit={audit}");
}

#[tokio::test]
async fn fhir_create_get_without_cdr_link() {
    let token = "fhir-ok-token";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "jane-1", None).await;
    let create = client()
        .post(format!("http://{addr}/v1/fhir/Patient"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "jane-1",
            "purpose": "care_provision",
            "link_cdr": false,
            "resource": {
                "resourceType": "Patient",
                "id": "jane-1",
                "name": [{"family": "Doe", "given": ["Jane"]}]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        create.status(),
        201,
        "body={}",
        create.text().await.unwrap()
    );
    let body: Value = create.json().await.unwrap();
    let id = body["id"].as_str().unwrap();
    let get = client()
        .get(format!("http://{addr}/v1/fhir/Patient/{id}"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/h3")
        .header("X-Solum-Capability", "solum:cdr:read")
        .header("X-Solum-Subject", "jane-1")
        .header("X-Solum-Purpose", "care_provision")
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 200, "body={}", get.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.fhir.created"));
}

#[tokio::test]
async fn aql_rejected_without_select() {
    let ehr = spawn_mock_ehrbase().await;
    let token = "aql-token";
    let (addr, _dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, format!("http://{ehr}/ehrbase")).await;
    grant_care_provision_http(&addr.to_string(), token).await;
    let res = client()
        .post(format!("http://{addr}/v1/cdr/aql"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:read"],
            "subject": "patient/42",
            "purpose": "care_provision",
            "q": "DELETE FROM EHR"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400, "body={}", res.text().await.unwrap());
}

#[tokio::test]
async fn aql_allowlisted_ok() {
    let ehr = spawn_mock_ehrbase().await;
    let token = "aql-ok-token";
    let (addr, dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, format!("http://{ehr}/ehrbase")).await;
    grant_care_provision_http(&addr.to_string(), token).await;
    let res = client()
        .post(format!("http://{addr}/v1/cdr/aql"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:read"],
            "subject": "patient/42",
            "purpose": "care_provision",
            "q": "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c WHERE e/ehr_id/value = 'patient/42'"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "body={}", res.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.aql.executed"));
}

#[tokio::test]
async fn subject_link_round_trip() {
    let token = "subject-link-token";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "subj-42", None).await;
    let put = client()
        .post(format!("http://{addr}/v1/cdr/subject-link"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "purpose": "care_provision",
            "solum_subject_id": "subj-42",
            "ferrum_drs_id": "drs.example/abc",
            "phenopacket_id": "ppkt-1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(put.status(), 200, "body={}", put.text().await.unwrap());
    let get = client()
        .get(format!("http://{addr}/v1/cdr/subject-link/subj-42"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/h3")
        .header("X-Solum-Capability", "solum:cdr:read")
        .header("X-Solum-Subject", "subj-42")
        .header("X-Solum-Purpose", "care_provision")
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 200, "body={}", get.text().await.unwrap());
    let body: Value = get.json().await.unwrap();
    assert_eq!(body["ferrum_drs_id"], "drs.example/abc");
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.subject_link.upserted"));
}

#[tokio::test]
async fn fhir_patient_auto_subject_link() {
    let token = "patient-bridge-token";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "bridge-patient-1", None).await;
    let create = client()
        .post(format!("http://{addr}/v1/fhir/Patient"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "bridge-patient-1",
            "purpose": "care_provision",
            "link_cdr": false,
            "resource": {
                "resourceType": "Patient",
                "id": "bridge-patient-1",
                "name": [{"family": "Bridge"}]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        create.status(),
        201,
        "body={}",
        create.text().await.unwrap()
    );
    let get = client()
        .get(format!(
            "http://{addr}/v1/cdr/subject-link/bridge-patient-1"
        ))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/h3")
        .header("X-Solum-Capability", "solum:cdr:read")
        .header("X-Solum-Subject", "bridge-patient-1")
        .header("X-Solum-Purpose", "care_provision")
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 200, "body={}", get.text().await.unwrap());
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.subject_link.upserted"));
}

#[tokio::test]
async fn dual_write_ok_without_cdr() {
    let token = "dual-ok-token";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "dw-1", None).await;
    let res = client()
        .post(format!("http://{addr}/v1/migrate/dual-write"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "dw-1",
            "purpose": "care_provision",
            "link_cdr": false,
            "source": "legacy-his",
            "resource": {
                "resourceType": "Patient",
                "id": "dw-1",
                "name": [{"family": "Mirror"}]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201, "body={}", res.text().await.unwrap());
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["dead_lettered"], false);
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.dual_write.ok"));
}

#[tokio::test]
async fn dual_write_dead_letters_on_cdr_failure() {
    let token = "dual-dl-token";
    let (addr, dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, "http://127.0.0.1:1/ehrbase".into()).await;
    grant_care_provision_for(&addr.to_string(), token, "c-fail", None).await;
    let res = client()
        .post(format!("http://{addr}/v1/migrate/dual-write"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "c-fail",
            "purpose": "care_provision",
            "link_cdr": true,
            "source": "legacy-his",
            "resource": {
                "resourceType": "Condition",
                "id": "c-fail",
                "subject": {"reference": "Patient/c-fail"},
                "code": {"text": "test"}
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 202, "body={}", res.text().await.unwrap());
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["dead_lettered"], true);
    let dl = std::fs::read_to_string(dir.path().join("dual_write_dead_letter.jsonl")).unwrap();
    assert!(!dl.is_empty(), "dead-letter file empty");
    assert!(
        !dl.contains("c-fail"),
        "dead-letter must not contain plaintext subject: {dl}"
    );
    assert!(
        dl.contains("ciphertext"),
        "dead-letter must be a Crypt4GH envelope: {dl}"
    );
    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("cdr.dual_write.dead_lettered"));
}

fn kenya_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/kenya-dpa.toml")
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kenya_dpa_refuses_ephemeral_even_with_allow_env() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_ALLOW_EPHEMERAL", "1");
    std::env::set_var("SOLUM_STORAGE_REGION", "KE");
    let dir = tempdir().unwrap();
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: kenya_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "kenya-eph".into(),
        keys_dir: None,
        ephemeral: true,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let err = match build_state(&config).await {
        Ok(_) => {
            std::env::remove_var("SOLUM_STORAGE_REGION");
            panic!("kenya-dpa must refuse EphemeralTest");
        }
        Err(e) => e,
    };
    std::env::remove_var("SOLUM_STORAGE_REGION");
    assert!(
        err.to_lowercase().contains("ephemeral")
            || err.to_lowercase().contains("custody")
            || err.to_lowercase().contains("startup")
            || err.contains("kenya-dpa"),
        "err={err}"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kenya_dpa_refuses_wrong_storage_region() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_STORAGE_REGION", "EU");
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, "ke/wrong-region");
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: kenya_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "kenya-region".into(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    let err = match build_state(&config).await {
        Ok(_) => {
            std::env::remove_var("SOLUM_STORAGE_REGION");
            panic!("kenya-dpa must refuse EU storage_region");
        }
        Err(e) => e,
    };
    std::env::remove_var("SOLUM_STORAGE_REGION");
    assert!(
        err.contains("storage_region") || err.contains("EU") || err.contains("startup"),
        "err={err}"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kenya_dpa_customer_held_starts_with_ke_region() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_STORAGE_REGION", "KE");
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, "ke/ok");
    let (jwks_src, _jwt, jwks_hold) = mint_rsa_jwks_and_token(&["solum-audit-ops"]);
    let jwks = dir.path().join("jwks.json");
    std::fs::copy(&jwks_src, &jwks).unwrap();
    drop(jwks_hold);
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: kenya_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "kenya-ok".into(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: Some(org_iam_mapping()),
        jwks_url: None,
        jwks_file: Some(jwks),
        oidc_issuer: Some(TEST_OIDC_ISSUER.into()),
        oidc_audience: Some(TEST_OIDC_AUD.into()),
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    };
    build_state(&config)
        .await
        .expect("kenya CustomerHeld + KE + org-IAM");
    std::env::remove_var("SOLUM_STORAGE_REGION");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn eu_ehds_refuses_without_org_iam() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("SOLUM_STORAGE_REGION", "EU");
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, "eu/no-iam");
    let err = match build_state(&SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "tok".into(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    })
    .await
    {
        Ok(_) => panic!("eu-ehds must require org-IAM"),
        Err(e) => e,
    };
    assert!(
        err.contains("org-iam") || err.contains("client-asserted"),
        "err={err}"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn eu_ehds_refuses_without_region_attestation() {
    let _guard = env_lock().lock().unwrap();
    std::env::remove_var("SOLUM_STORAGE_REGION");
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, "eu/no-region");
    let (jwks_src, _jwt, jwks_hold) = mint_rsa_jwks_and_token(&["solum-consent-ops"]);
    let jwks = dir.path().join("jwks.json");
    std::fs::copy(&jwks_src, &jwks).unwrap();
    drop(jwks_hold);
    let err = match build_state(&SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: "tok".into(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        wrapped_keys_dir: None,
        org_iam_config: Some(org_iam_mapping()),
        jwks_url: None,
        jwks_file: Some(jwks),
        oidc_issuer: Some(TEST_OIDC_ISSUER.into()),
        oidc_audience: Some(TEST_OIDC_AUD.into()),
        ehrbase_url: None,
        cdr_template_opt: None,
        fhir_store: None,
        subject_link_store: None,
        dual_write_dead_letter: None,
    })
    .await
    {
        Ok(_) => panic!("eu-ehds must require SOLUM_STORAGE_REGION"),
        Err(e) => e,
    };
    assert!(
        err.contains("SOLUM_STORAGE_REGION") || err.contains("attestation"),
        "err={err}"
    );
}

#[tokio::test]
async fn audit_export_without_capability_is_forbidden() {
    let token = "audit-export-deny";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    let export = client()
        .get(format!("http://{addr}/v1/audit/export"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/7")
        .header("X-Solum-Capability", "solum:consent:grant")
        .send()
        .await
        .unwrap();
    assert_eq!(
        export.status(),
        403,
        "body={}",
        export.text().await.unwrap()
    );
}

#[tokio::test]
async fn fhir_get_rejects_cross_subject_idor() {
    let token = "fhir-idor";
    let (addr, _dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "jane-1", None).await;
    grant_care_provision_for(&addr.to_string(), token, "patient/99", None).await;
    let create = client()
        .post(format!("http://{addr}/v1/fhir/Patient"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "jane-1",
            "purpose": "care_provision",
            "link_cdr": false,
            "resource": {
                "resourceType": "Patient",
                "id": "jane-1",
                "name": [{"family": "Doe"}]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        create.status(),
        201,
        "body={}",
        create.text().await.unwrap()
    );
    let get = client()
        .get(format!("http://{addr}/v1/fhir/Patient/jane-1"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .header("X-Solum-Actor", "practitioner/h3")
        .header("X-Solum-Capability", "solum:cdr:read")
        .header("X-Solum-Subject", "patient/99")
        .header("X-Solum-Purpose", "care_provision")
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 403, "body={}", get.text().await.unwrap());
}

#[tokio::test]
async fn aql_without_quoted_subject_is_rejected() {
    let ehr = spawn_mock_ehrbase().await;
    let token = "aql-idor";
    let (addr, _dir) =
        spawn_ephemeral_sidecar_with_ehrbase(token, format!("http://{ehr}/ehrbase")).await;
    grant_care_provision_http(&addr.to_string(), token).await;
    let res = client()
        .post(format!("http://{addr}/v1/cdr/aql"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:read"],
            "subject": "patient/42",
            "purpose": "care_provision",
            "q": "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400, "body={}", res.text().await.unwrap());
}

#[tokio::test]
async fn fhir_store_jsonl_is_crypt4gh_not_plaintext() {
    let token = "fhir-enc-store";
    let (addr, dir) = spawn_ephemeral_sidecar(token).await;
    grant_care_provision_for(&addr.to_string(), token, "enc-p", None).await;
    let create = client()
        .post(format!("http://{addr}/v1/fhir/Patient"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "actor": "practitioner/h3",
            "capability": ["solum:cdr:write"],
            "subject": "enc-p",
            "purpose": "care_provision",
            "link_cdr": false,
            "resource": {
                "resourceType": "Patient",
                "id": "enc-p",
                "name": [{"family": "SecretFamily"}]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        create.status(),
        201,
        "body={}",
        create.text().await.unwrap()
    );
    let raw = std::fs::read_to_string(dir.path().join("fhir_store.jsonl")).unwrap();
    assert!(
        !raw.contains("SecretFamily"),
        "FHIR JSONL must not store plaintext: {raw}"
    );
    assert!(
        raw.contains("ciphertext"),
        "expected Crypt4GH envelope: {raw}"
    );
}

#[test]
fn non_loopback_bind_without_tls_is_refused() {
    let bind: std::net::SocketAddr = "0.0.0.0:8787".parse().unwrap();
    let err = validate_listen_bind(bind, false).unwrap_err();
    assert!(err.contains("non-loopback"), "{err}");
}
