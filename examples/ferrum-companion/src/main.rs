//! Mode B — Ferrum-companion reference (Sprint 1 + optional Sprint 4 storage).
//!
//! Proves Crypt4GH format compatibility between a Ferrum-style direct
//! `crypt4gh` encrypt path and Solum's `encrypt_field` for the same key
//! material, plus a zero-logic smoke that `ferrum_core::auth::AuthClaims`
//! is constructible at the pinned revision.
//!
//! With `--features storage-backend`: also proves
//! `Deployment::encrypt_field_and_store` / `read_and_decrypt_field` against
//! Ferrum `LocalStorage` (async API; runtime only in this binary).

use std::collections::HashSet;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::ExitCode;

use crypt4gh::keys::{generate_private_key, get_public_key_from_private_key};
use crypt4gh::{decrypt, encrypt, Keys};
use solum_crypto::ferrum_core::auth::AuthClaims;
use solum_crypto::{
    decrypt_field, encrypt_field, CustomerHeldKeyProvider, EncryptedField, FieldCategoryGate,
    KeyRef, ENVELOPE_ALGORITHM,
};
use solum_profiles::load_profile;

#[cfg(not(feature = "storage-backend"))]
fn main() -> ExitCode {
    match run_sync() {
        Ok(()) => {
            println!("ok: ferrum-companion reference deployment (Mode B) passed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "storage-backend")]
#[tokio::main]
async fn main() -> ExitCode {
    match run_async().await {
        Ok(()) => {
            println!("ok: ferrum-companion reference deployment (Mode B) passed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_sync() -> Result<(), String> {
    smoke_auth_claims()?;
    crypt4gh_format_interop()?;
    Ok(())
}

#[cfg(feature = "storage-backend")]
async fn run_async() -> Result<(), String> {
    run_sync()?;
    storage_round_trip().await?;
    Ok(())
}

/// Sprint 4: encrypt → Ferrum LocalStorage → read → decrypt via Deployment.
#[cfg(feature = "storage-backend")]
async fn storage_round_trip() -> Result<(), String> {
    use ferrum_storage::LocalStorage;
    use solum_core::crypto::CustomerHeldKeyProvider;
    use solum_core::identity;
    use solum_core::{example_eu_runtime, Deployment, SolumActor};

    let work = tempfile::tempdir().map_err(|e| e.to_string())?;
    let profile_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml");
    let key_ref = KeyRef::new("companion/storage-1");
    let (pubkey, privkey) =
        solum_core::crypto::generate_operator_keypair().map_err(|e| e.to_string())?;
    let mut keys = CustomerHeldKeyProvider::new();
    keys.register_customer_keypair(key_ref.clone(), pubkey, privkey)
        .map_err(|e| e.to_string())?;

    let local = LocalStorage::new(work.path().join("objects")).map_err(|e| e.to_string())?;
    let mut deployment = Deployment::open(
        &profile_path,
        &example_eu_runtime(),
        work.path().join("audit.jsonl"),
        work.path().join("consent.jsonl"),
        keys,
    )
    .map_err(|e| e.to_string())?
    .with_storage(local);

    let actor = SolumActor::standalone(
        "practitioner/7",
        vec![
            identity::CAP_CONSENT_GRANT.into(),
            identity::CAP_CRYPTO_ENCRYPT.into(),
            identity::CAP_CRYPTO_DECRYPT.into(),
        ],
    );
    deployment
        .grant_consent_as("patient/42", "care_provision", vec![], &actor)
        .map_err(|e| e.to_string())?;

    let plain = b"patient-summary-ferrum-storage-demo";
    let storage_key = "fields/patient_summary/companion-1.json";
    deployment
        .encrypt_field_and_store(
            "patient_summary",
            plain,
            &key_ref,
            &actor,
            "patient/42",
            "care_provision",
            storage_key,
        )
        .await
        .map_err(|e| e.to_string())?;
    let out = deployment
        .read_and_decrypt_field(
            storage_key,
            &key_ref,
            &actor,
            "patient/42",
            "care_provision",
        )
        .await
        .map_err(|e| e.to_string())?;
    if out != plain {
        return Err("storage round-trip plaintext mismatch".into());
    }

    println!(
        "ok: LocalStorage encrypt_field_and_store ↔ read_and_decrypt_field for patient_summary"
    );
    Ok(())
}

/// Smoke: construct and query `AuthClaims` (no JWT verification — Sprint 5).
///
/// Inspected API at ferrum-core pin `27a6a8e9…`: `AuthClaims` is a public
/// enum (`Jwt` / `Passport`) with `Debug`+`Clone` but **no** `Serialize`/
/// `Deserialize`. Fixture construction uses the `Jwt` variant fields
/// directly (same shape Ferrum tests use in `auth.rs`).
fn smoke_auth_claims() -> Result<(), String> {
    let claims = AuthClaims::Jwt {
        sub: "researcher@example.org".into(),
        iss: Some("https://passports.example/issuer".into()),
        exp: 4_102_444_800, // far-future placeholder
        jti: Some("smoke-jti-1".into()),
        scope: Some("drs.read ferrum:analyst".into()),
        raw_token: None,
    };

    if claims.sub() != Some("researcher@example.org") {
        return Err(format!("unexpected sub: {:?}", claims.sub()));
    }
    if claims.issuer() != Some("https://passports.example/issuer") {
        return Err(format!("unexpected issuer: {:?}", claims.issuer()));
    }
    if !claims.has_scope("drs.read") {
        return Err("expected has_scope(drs.read)".into());
    }
    if claims.is_admin() {
        return Err("Jwt claims must not report is_admin".into());
    }

    println!("ok: AuthClaims Jwt fixture constructible (sub/issuer/scope)");
    Ok(())
}

/// Same keypair → Ferrum-style crypt4gh encrypt + Solum encrypt_field;
/// cross-decrypt both ways.
fn crypt4gh_format_interop() -> Result<(), String> {
    let plain = b"patient-summary-shared-format-demo";

    // Key material via crypt4gh (the library Ferrum and Solum both use).
    let privkey = generate_private_key();
    let pubkey = get_public_key_from_private_key(&privkey)
        .map_err(|e| format!("pubkey from privkey: {e}"))?;

    // --- Ferrum-side path: direct crypt4gh encrypt (genomic-object style) ---
    let ferrum_ciphertext = crypt4gh_encrypt_for_pubkey(&pubkey, plain)?;
    if !ferrum_ciphertext.starts_with(b"crypt4gh") {
        return Err("Ferrum-path ciphertext missing crypt4gh magic".into());
    }

    // --- Solum-side path: register same keypair, encrypt_field ---
    let profile_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml");
    let profile = load_profile(&profile_path).map_err(|e| e.to_string())?;
    let gate = FieldCategoryGate::new(&profile.encryption.required_field_categories);
    let key_ref = KeyRef::new("companion/shared-1");

    let mut provider = CustomerHeldKeyProvider::new();
    provider
        .register_customer_keypair(key_ref.clone(), pubkey.clone(), privkey.clone())
        .map_err(|e| e.to_string())?;

    let solum_field = encrypt_field(&gate, &provider, "patient_summary", plain, &key_ref)
        .map_err(|e| e.to_string())?;
    if !solum_field.ciphertext.starts_with(b"crypt4gh") {
        return Err("Solum ciphertext missing crypt4gh magic".into());
    }

    // Solum decrypts Ferrum-path bytes (wrapped as EncryptedField).
    let wrapped_ferrum = EncryptedField {
        category: "patient_summary".into(),
        key_ref: key_ref.clone(),
        algorithm: ENVELOPE_ALGORITHM.into(),
        ciphertext: ferrum_ciphertext.clone(),
    };
    let from_ferrum =
        decrypt_field(&provider, &wrapped_ferrum, &key_ref).map_err(|e| e.to_string())?;
    if from_ferrum != plain {
        return Err("Solum failed to decrypt Ferrum-path ciphertext".into());
    }

    // Raw crypt4gh decrypts Solum ciphertext.
    let keys = vec![Keys {
        method: 0,
        privkey: privkey[..32].to_vec(),
        recipient_pubkey: pubkey,
    }];
    let from_solum = crypt4gh_decrypt(&keys, &solum_field.ciphertext)?;
    if from_solum != plain {
        return Err("crypt4gh failed to decrypt Solum ciphertext".into());
    }

    println!("ok: Crypt4GH interop (Ferrum-path ↔ Solum encrypt_field) for patient_summary");
    Ok(())
}

/// Mirror of Solum's private `encrypt_crypt4gh` / Ferrum's recipient-pubkey pattern:
/// ephemeral sender key + recipient pubkey via the shared `crypt4gh` crate.
fn crypt4gh_encrypt_for_pubkey(
    recipient_pubkey: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let ephemeral = generate_private_key();
    let mut recipients = HashSet::new();
    recipients.insert(Keys {
        method: 0,
        privkey: ephemeral[..32].to_vec(),
        recipient_pubkey: recipient_pubkey.to_vec(),
    });
    let mut reader = Cursor::new(plaintext.to_vec());
    let mut writer = Vec::new();
    encrypt(&recipients, &mut reader, &mut writer, 0, None)
        .map_err(|e| format!("crypt4gh encrypt: {e}"))?;
    Ok(writer)
}

fn crypt4gh_decrypt(keys: &[Keys], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = Cursor::new(ciphertext.to_vec());
    let mut writer = Vec::new();
    decrypt(keys, &mut reader, &mut writer, 0, None, &None)
        .map_err(|e| format!("crypt4gh decrypt: {e}"))?;
    Ok(writer)
}
