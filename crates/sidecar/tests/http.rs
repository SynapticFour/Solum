//! HTTP integration tests for `solum-sidecar` (axum + reqwest against a free port).

use std::net::SocketAddr;
use std::path::PathBuf;

use base64::Engine;
use serde_json::Value;
use solum_sidecar::{
    app_router, build_state, SidecarConfig, EPHEMERAL_WARNING_HEADER, SIDECAR_TOKEN_HEADER,
};
use tempfile::tempdir;

fn eu_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
}

async fn spawn_sidecar(token: &str) -> (SocketAddr, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let config = SidecarConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        profile: eu_profile(),
        audit: dir.path().join("audit.jsonl"),
        consent_store: dir.path().join("consent.jsonl"),
        token: token.to_string(),
    };
    let state = build_state(&config).expect("build_state");
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
    let (addr, _dir) = spawn_sidecar(token).await;
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
    let (addr, dir) = spawn_sidecar(token).await;
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
    let (addr, _dir) = spawn_sidecar(token).await;
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
    let (addr, _dir) = spawn_sidecar(token).await;
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
    let (addr, _dir) = spawn_sidecar(token).await;
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
    let (addr, _dir) = spawn_sidecar(token).await;
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
    let (addr, _dir) = spawn_sidecar(token).await;

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
