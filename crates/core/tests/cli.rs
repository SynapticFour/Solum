//! Integration tests for the `solum` CLI binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

fn eu_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
}

fn dev_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/dev-local.toml")
}

fn solum() -> assert_cmd::Command {
    cargo_bin_cmd!("solum")
}

fn write_keypair(dir: &Path, key_ref: &str) -> PathBuf {
    let out = dir.join("customer.keypair.json");
    solum()
        .args([
            "crypto",
            "keygen",
            "--key-ref",
            key_ref,
            "--out",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();
    out
}

fn grant_care_provision(dir: &std::path::Path) {
    grant_care_provision_on(dir, eu_profile());
}

fn grant_care_provision_on(dir: &std::path::Path, profile: PathBuf) {
    solum()
        .args([
            "consent",
            "grant",
            "--profile",
            profile.to_str().unwrap(),
            "--audit",
            dir.join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.join("consent.jsonl").to_str().unwrap(),
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:consent:grant",
            "--scope",
            "patient_summary",
        ])
        .assert()
        .success();
}

fn grant(dir: &Path, subject: &str, purpose: &str) {
    solum()
        .args([
            "consent",
            "grant",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.join("consent.jsonl").to_str().unwrap(),
            "--subject",
            subject,
            "--purpose",
            purpose,
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:consent:grant",
            "--scope",
            "patient_summary",
        ])
        .assert()
        .success();
}

fn status_stdout(dir: &Path, subject: &str, purpose: &str) -> String {
    let assert = solum()
        .args([
            "consent",
            "status",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--consent-store",
            dir.join("consent.jsonl").to_str().unwrap(),
            "--subject",
            subject,
            "--purpose",
            purpose,
        ])
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone())
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn consent_grant_then_status_granted() {
    let dir = tempdir().unwrap();
    grant(dir.path(), "patient/42", "care_provision");
    assert_eq!(
        status_stdout(dir.path(), "patient/42", "care_provision"),
        "granted"
    );
}

#[test]
fn consent_grant_revoke_then_status_revoked() {
    let dir = tempdir().unwrap();
    grant(dir.path(), "patient/42", "care_provision");
    solum()
        .args([
            "consent",
            "revoke",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--actor",
            "patient/42",
            "--capability",
            "solum:consent:revoke",
        ])
        .assert()
        .success();
    assert_eq!(
        status_stdout(dir.path(), "patient/42", "care_provision"),
        "revoked"
    );
}

#[test]
fn consent_grant_without_capability_is_denied() {
    let dir = tempdir().unwrap();
    let assert = solum()
        .args([
            "consent",
            "grant",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--actor",
            "practitioner/7",
            "--scope",
            "patient_summary",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("lacks required capability") || stderr.contains("solum:consent:grant"),
        "fail-closed without --capability: {stderr}"
    );
    assert_eq!(
        status_stdout(dir.path(), "patient/42", "care_provision"),
        "unknown"
    );
}

#[test]
fn crypto_encrypt_decrypt_customer_held_round_trip() {
    let dir = tempdir().unwrap();
    let key_ref = "customer/eval-1";
    let keypair = write_keypair(dir.path(), key_ref);
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    let plain_out = dir.path().join("plain-out.txt");
    fs::write(&plain_in, b"patient-summary-demo").unwrap();
    grant_care_provision(dir.path());

    let enc = solum()
        .args([
            "crypto",
            "encrypt",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "patient_summary",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            key_ref,
            "--keypair",
            keypair.to_str().unwrap(),
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&enc.get_output().stderr);
    assert!(
        stderr.contains("CustomerHeld"),
        "encrypt must note CustomerHeld path: {stderr}"
    );
    assert!(
        !stderr.contains("EphemeralTestKeyProvider"),
        "CustomerHeld path must not print ephemeral warning: {stderr}"
    );

    solum()
        .args([
            "crypto",
            "decrypt",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            key_ref,
            "--keypair",
            keypair.to_str().unwrap(),
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:decrypt",
            "--in",
            enc_out.to_str().unwrap(),
            "--out",
            plain_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read(&plain_out).unwrap(), b"patient-summary-demo");
}

#[test]
fn crypto_ephemeral_refused_on_eu_profile_even_with_env() {
    let dir = tempdir().unwrap();
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    fs::write(&plain_in, b"x").unwrap();

    let assert = solum()
        .env("SOLUM_ALLOW_EPHEMERAL", "1")
        .args([
            "crypto",
            "encrypt",
            "--ephemeral",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "patient_summary",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("startup refused")
            || stderr.contains("CustodyNotAllowed")
            || stderr.contains("ephemeral"),
        "eu-ehds must refuse EphemeralTest custody: {stderr}"
    );
    assert!(!enc_out.exists());
}

#[test]
fn crypto_ephemeral_requires_allow_env() {
    let dir = tempdir().unwrap();
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    fs::write(&plain_in, b"x").unwrap();

    let assert = solum()
        .env_remove("SOLUM_ALLOW_EPHEMERAL")
        .args([
            "crypto",
            "encrypt",
            "--ephemeral",
            "--profile",
            dev_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "patient_summary",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("SOLUM_ALLOW_EPHEMERAL"),
        "must require env gate: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn crypto_ephemeral_sidecar_is_mode_0600_on_dev_profile() {
    use std::os::unix::fs::MetadataExt;

    let dir = tempdir().unwrap();
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    fs::write(&plain_in, b"x").unwrap();

    grant_care_provision_on(dir.path(), dev_profile());

    solum()
        .env("SOLUM_ALLOW_EPHEMERAL", "1")
        .args([
            "crypto",
            "encrypt",
            "--ephemeral",
            "--profile",
            dev_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "patient_summary",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut sidecar = enc_out.into_os_string();
    sidecar.push(".ephemeral-keypair.json");
    let sidecar = PathBuf::from(sidecar);
    let mode = fs::metadata(&sidecar).unwrap().mode() & 0o777;
    assert_eq!(
        mode,
        0o600,
        "sidecar {} mode was {mode:o}",
        sidecar.display()
    );
}

#[test]
fn audit_verify_ok_after_consent_and_crypto() {
    let dir = tempdir().unwrap();
    grant(dir.path(), "patient/42", "care_provision");

    let key_ref = "customer/eval-1";
    let keypair = write_keypair(dir.path(), key_ref);
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    fs::write(&plain_in, b"x").unwrap();
    solum()
        .args([
            "crypto",
            "encrypt",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "patient_summary",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            key_ref,
            "--keypair",
            keypair.to_str().unwrap(),
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let assert = solum()
        .args([
            "audit",
            "verify",
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "ok");
}

#[test]
fn crypto_encrypt_rejects_unknown_category_without_panic() {
    let dir = tempdir().unwrap();
    let key_ref = "customer/eval-1";
    let keypair = write_keypair(dir.path(), key_ref);
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    fs::write(&plain_in, b"x").unwrap();

    let assert = solum()
        .args([
            "crypto",
            "encrypt",
            "--profile",
            eu_profile().to_str().unwrap(),
            "--audit",
            dir.path().join("audit.jsonl").to_str().unwrap(),
            "--consent-store",
            dir.path().join("consent.jsonl").to_str().unwrap(),
            "--category",
            "marketing_segment",
            "--subject",
            "patient/42",
            "--purpose",
            "care_provision",
            "--key-ref",
            key_ref,
            "--keypair",
            keypair.to_str().unwrap(),
            "--actor",
            "practitioner/7",
            "--capability",
            "solum:crypto:encrypt",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("marketing_segment") || stderr.contains("fatal"),
        "expected clear rejection, got: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at") && !stderr.contains("RUST_BACKTRACE"),
        "unknown category must not panic: {stderr}"
    );
    assert!(!enc_out.exists(), "failed encrypt must not write --out");
}

#[test]
fn check_still_works() {
    // Sanity: existing verify.sh entrypoint remains valid under clap.
    let status = Command::new(assert_cmd::cargo::cargo_bin!("solum"))
        .args(["check", "--profile", eu_profile().to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn check_refuses_ephemeral_custody_on_eu_profile() {
    let assert = solum()
        .env("SOLUM_KEY_CUSTODY", "ephemeral_test")
        .args(["check", "--profile", eu_profile().to_str().unwrap()])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("startup refused") || stderr.contains("ephemeral"),
        "expected custody refusal: {stderr}"
    );
}
