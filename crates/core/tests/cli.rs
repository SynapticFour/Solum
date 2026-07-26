//! Integration tests for the `solum` CLI binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

fn eu_profile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
}

fn solum() -> assert_cmd::Command {
    cargo_bin_cmd!("solum")
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
        ])
        .assert()
        .success();
    assert_eq!(
        status_stdout(dir.path(), "patient/42", "care_provision"),
        "revoked"
    );
}

#[test]
fn crypto_encrypt_decrypt_round_trip() {
    let dir = tempdir().unwrap();
    let plain_in = dir.path().join("plain.txt");
    let enc_out = dir.path().join("field.json");
    let plain_out = dir.path().join("plain-out.txt");
    fs::write(&plain_in, b"patient-summary-demo").unwrap();

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
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
            "--in",
            plain_in.to_str().unwrap(),
            "--out",
            enc_out.to_str().unwrap(),
        ])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&enc.get_output().stderr);
    assert!(
        stderr.contains("EphemeralTestKeyProvider"),
        "encrypt must print the ephemeral-key warning: {stderr}"
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
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
            "--in",
            enc_out.to_str().unwrap(),
            "--out",
            plain_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read(&plain_out).unwrap(), b"patient-summary-demo");
}

#[cfg(unix)]
#[test]
fn crypto_encrypt_sidecar_is_mode_0600() {
    use std::os::unix::fs::MetadataExt;

    let dir = tempdir().unwrap();
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
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
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
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
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
            "--key-ref",
            "ephemeral/cli-1",
            "--actor",
            "practitioner/7",
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
    // Smoke-check: process exited via ExitCode, not an abort/panic payload.
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
