//! HTTP integration tests for `solum-sidecar` (axum + reqwest against a free port).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use serde_json::Value;
use solum_core::crypto::generate_operator_keypair;
use solum_sidecar::{
    app_router, build_state, KeypairFile, SidecarConfig, CUSTOMER_HELD_KEY_NOTE,
    EPHEMERAL_WARNING_HEADER, SIDECAR_TOKEN_HEADER,
};
use tempfile::tempdir;

/// Serialize env mutations for ephemeral gate tests (process-wide `SOLUM_ALLOW_EPHEMERAL`).
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
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
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

/// CustomerHeld sidecar: `--keys-dir` with a pre-registered keypair.
async fn spawn_customer_held_sidecar(
    token: &str,
    key_ref: &str,
) -> (SocketAddr, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let keys_dir = dir.path().join("keys");
    std::fs::create_dir_all(&keys_dir).unwrap();
    write_keypair_file(&keys_dir, key_ref);
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
        keys_dir: Some(keys_dir),
        ephemeral: false,
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
    };
    let state = build_state(&config)
        .await
        .expect("build_state customer-held");
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, dir)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
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
    let plain = b"patient-summary-demo";
    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
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
    let key_ref = "ephemeral/reuse-1";
    let plain_a = b"first-plaintext-block";
    let plain_b = b"second-plaintext-block";

    let enc_a = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
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
        .send()
        .await
        .unwrap();
    assert_eq!(verify.status(), 200);
    let body: Value = verify.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let export = client()
        .get(format!("http://{addr}/v1/audit/export"))
        .header(SIDECAR_TOKEN_HEADER, token)
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
    let (addr, _dir) = spawn_customer_held_sidecar(token, key_ref).await;
    let plain = b"customer-held-plaintext";

    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
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
        .json(&serde_json::json!({
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
    let (addr, _dir) = spawn_customer_held_sidecar(token, "customer/known-1").await;

    let enc = client()
        .post(format!("http://{addr}/v1/crypto/encrypt"))
        .header(SIDECAR_TOKEN_HEADER, token)
        .json(&serde_json::json!({
            "category": "patient_summary",
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
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
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
        org_iam_config: None,
        jwks_url: None,
        jwks_file: None,
        oidc_issuer: None,
        oidc_audience: None,
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
    use base64::Engine;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use rand::rngs::OsRng;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid);
    let token = encode(
        &header,
        &json!({
            "sub": "practitioner/org-iam",
            "exp": t + 3600,
            "groups": groups,
        }),
        &encoding,
    )
    .unwrap();
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
        org_iam_config: Some(mapping_path),
        jwks_url: None,
        jwks_file: Some(jwks_file),
        oidc_issuer: None,
        oidc_audience: None,
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
    let (addr, _dir) = spawn_org_iam_sidecar(token, jwks, mapping).await;
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
