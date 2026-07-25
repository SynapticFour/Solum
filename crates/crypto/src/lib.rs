//! Solum crypto layer: customer-held keys and clinical **field** envelope encryption.
//!
//! Reuses Ferrum sovereignty building blocks via a git-pinned `ferrum-core`
//! dependency (Lab Kit pattern). Clinical field AEAD and custody policy that
//! Ferrum does not provide belong **here**, not in Ferrum upstream.
//!
//! **Not Crypt4GH:** Ferrum’s Crypt4GH layer encrypts genomic *file/stream*
//! DRS objects. Solum uses the same AEAD family (ChaCha20-Poly1305) in a
//! compact serde envelope for clinical categories — see `docs/CRYPTO.md`.
//!
//! ## Customer-held keys
//!
//! When [`KeyCustody::CustomerHeld`] is active, Solum **never** generates or
//! persists the Key Encryption Key (KEK). Callers supply KEK material through
//! a [`KekProvider`] that only references external HSM/KMS keys by [`KeyRef`].
//!
//! ## Honest zero-knowledge path
//!
//! Field encrypt/decrypt necessarily touches plaintext in process memory.
//! This crate does not claim cryptographic zero-knowledge for those steps;
//! accountability rests on customer-held KEKs plus auditability elsewhere.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Git revision pinned in `Cargo.toml` (mirror `config/ci/ferrum-revision.txt`).
pub const FERRUM_GIT_REV: &str = "27a6a8e9a719fd1a171da28b20462a777f95cf65";

/// Upstream repository URL.
pub const FERRUM_GIT_URL: &str = "https://github.com/SynapticFour/Ferrum.git";

/// Envelope algorithm identifier stored on [`EncryptedField`].
pub const ENVELOPE_ALGORITHM: &str = "chacha20poly1305-envelope-v1";

const DEK_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// Re-export Ferrum shared types for Solum integrators (no logic duplication).
pub use ferrum_core;

/// How encryption keys are held for a deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCustody {
    /// Keys never leave the customer's HSM / KMS boundary.
    CustomerHeld,
    /// Keys managed by Solum operator on behalf of the customer (restricted).
    OperatorHeld,
    /// Ephemeral / test-only keys — never for regulated production.
    EphemeralTest,
}

/// Declared key-management posture of a running deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyManagementConfig {
    pub custody: KeyCustody,
    /// Optional KMS / HSM provider identifier (e.g. `aws-kms-eu-central-1`).
    pub provider: Option<String>,
}

/// Opaque reference to a customer-held (or test) KEK. Solum stores the id only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyRef {
    pub id: String,
}

impl KeyRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Serde-serialisable envelope: DEK wraps the field; KEK wraps the DEK.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedField {
    pub category: String,
    pub key_ref: KeyRef,
    pub algorithm: String,
    /// 12-byte ChaCha20-Poly1305 nonce (raw bytes).
    pub nonce: Vec<u8>,
    /// Ciphertext including the Poly1305 tag.
    pub ciphertext: Vec<u8>,
    /// DEK encrypted under the KEK referenced by `key_ref`.
    pub wrapped_dek: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key custody {actual:?} is not allowed by the active jurisdiction profile (allowed: {allowed:?})")]
    CustodyNotAllowed {
        actual: KeyCustody,
        allowed: Vec<KeyCustody>,
    },
    #[error("field category '{category}' is not in the profile required_field_categories (allowed: {allowed:?})")]
    CategoryNotRecognised {
        category: String,
        allowed: Vec<String>,
    },
    #[error("unknown key reference '{0}'")]
    UnknownKeyRef(String),
    #[error("AEAD encryption failed")]
    Encrypt,
    #[error("AEAD decryption failed (wrong key or tampered ciphertext)")]
    Decrypt,
    #[error("invalid wrapped DEK or nonce length")]
    InvalidEnvelope,
    #[error("KEK provider refused operation: {0}")]
    Provider(String),
}

/// Wraps/unwraps data-encryption keys under a KEK that Solum does not own.
pub trait KekProvider {
    fn wrap_dek(&self, key_ref: &KeyRef, dek: &[u8; DEK_LEN]) -> Result<Vec<u8>, CryptoError>;
    fn unwrap_dek(
        &self,
        key_ref: &KeyRef,
        wrapped_dek: &[u8],
    ) -> Result<[u8; DEK_LEN], CryptoError>;
}

/// Customer-held KEK registry: keys are **registered by the customer**, never
/// generated or persisted by Solum. Simulates an external HSM/KMS lookup by
/// [`KeyRef`].
#[derive(Debug, Default)]
pub struct CustomerHeldKekProvider {
    /// In-process stand-in for HSM slots. Production deployments replace this
    /// type with a real KMS client; Solum still must not mint KEKs.
    keks: HashMap<String, [u8; DEK_LEN]>,
}

impl CustomerHeldKekProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a KEK that was generated **outside** Solum (HSM/KMS/customer).
    pub fn register_customer_kek(&mut self, key_ref: KeyRef, kek: [u8; DEK_LEN]) {
        self.keks.insert(key_ref.id, kek);
    }
}

impl KekProvider for CustomerHeldKekProvider {
    fn wrap_dek(&self, key_ref: &KeyRef, dek: &[u8; DEK_LEN]) -> Result<Vec<u8>, CryptoError> {
        let kek = self
            .keks
            .get(&key_ref.id)
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))?;
        aead_encrypt(kek, dek)
    }

    fn unwrap_dek(
        &self,
        key_ref: &KeyRef,
        wrapped_dek: &[u8],
    ) -> Result<[u8; DEK_LEN], CryptoError> {
        let kek = self
            .keks
            .get(&key_ref.id)
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))?;
        let plain = aead_decrypt(kek, wrapped_dek)?;
        let mut dek = [0u8; DEK_LEN];
        if plain.len() != DEK_LEN {
            return Err(CryptoError::InvalidEnvelope);
        }
        dek.copy_from_slice(&plain);
        Ok(dek)
    }
}

/// In-memory KEK provider for unit tests and local demos.
///
/// **Never for regulated production** — same posture as [`KeyCustody::EphemeralTest`].
/// Prefer [`CustomerHeldKekProvider`] with externally supplied KEKs for any
/// deployment that handles real clinical data.
#[derive(Debug, Default)]
pub struct EphemeralTestKekProvider {
    keks: HashMap<String, [u8; DEK_LEN]>,
}

impl EphemeralTestKekProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate and retain a throwaway KEK. **Test/demo only.**
    pub fn generate_test_kek(&mut self, key_ref: KeyRef) -> [u8; DEK_LEN] {
        tracing::warn!(
            key_ref = %key_ref.id,
            "EphemeralTestKekProvider::generate_test_kek — never for regulated production"
        );
        let mut kek = [0u8; DEK_LEN];
        OsRng.fill_bytes(&mut kek);
        self.keks.insert(key_ref.id, kek);
        kek
    }
}

impl KekProvider for EphemeralTestKekProvider {
    fn wrap_dek(&self, key_ref: &KeyRef, dek: &[u8; DEK_LEN]) -> Result<Vec<u8>, CryptoError> {
        let kek = self
            .keks
            .get(&key_ref.id)
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))?;
        aead_encrypt(kek, dek)
    }

    fn unwrap_dek(
        &self,
        key_ref: &KeyRef,
        wrapped_dek: &[u8],
    ) -> Result<[u8; DEK_LEN], CryptoError> {
        let kek = self
            .keks
            .get(&key_ref.id)
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))?;
        let plain = aead_decrypt(kek, wrapped_dek)?;
        let mut dek = [0u8; DEK_LEN];
        if plain.len() != DEK_LEN {
            return Err(CryptoError::InvalidEnvelope);
        }
        dek.copy_from_slice(&plain);
        Ok(dek)
    }
}

/// Gate that only allows categories listed in a jurisdiction profile's
/// `encryption.required_field_categories`.
///
/// Pass `profile.encryption.required_field_categories` from
/// `solum_profiles::JurisdictionProfile`. This crate does not depend on
/// `solum-profiles` at runtime (that crate already depends on us for
/// [`KeyCustody`]); the slice is the stable boundary.
#[derive(Debug, Clone, Copy)]
pub struct FieldCategoryGate<'a> {
    required_field_categories: &'a [String],
}

impl<'a> FieldCategoryGate<'a> {
    pub fn new(required_field_categories: &'a [String]) -> Self {
        Self {
            required_field_categories,
        }
    }
}

/// Smoketest that `ferrum-core` symbols resolve at link time.
pub fn ferrum_core_type_name() -> &'static str {
    std::any::type_name::<ferrum_core::FerrumError>()
}

/// Validate that runtime key custody matches what a jurisdiction profile allows.
pub fn validate_key_custody(
    runtime: &KeyManagementConfig,
    allowed: &[KeyCustody],
) -> Result<(), CryptoError> {
    if allowed.iter().any(|c| c == &runtime.custody) {
        Ok(())
    } else {
        Err(CryptoError::CustodyNotAllowed {
            actual: runtime.custody.clone(),
            allowed: allowed.to_vec(),
        })
    }
}

/// Ensure `category` appears in the profile's required encryption categories
/// (same style as `solum_consent::validate_purpose`).
pub fn validate_field_category(
    gate: &FieldCategoryGate<'_>,
    category: &str,
) -> Result<(), CryptoError> {
    if gate.required_field_categories.iter().any(|c| c == category) {
        Ok(())
    } else {
        Err(CryptoError::CategoryNotRecognised {
            category: category.to_string(),
            allowed: gate.required_field_categories.to_vec(),
        })
    }
}

/// Encrypt one clinical field under envelope encryption (fresh DEK per call).
pub fn encrypt_field(
    gate: &FieldCategoryGate<'_>,
    provider: &impl KekProvider,
    category: &str,
    plaintext: &[u8],
    key_ref: &KeyRef,
) -> Result<EncryptedField, CryptoError> {
    validate_field_category(gate, category)?;

    let mut dek = SensitiveKey::random();
    let ciphertext = aead_encrypt(&dek.0, plaintext)?;
    let (nonce, ct) = split_nonce_ciphertext(&ciphertext)?;
    let wrapped_dek = provider.wrap_dek(key_ref, &dek.0)?;
    dek.zeroize();

    Ok(EncryptedField {
        category: category.to_string(),
        key_ref: key_ref.clone(),
        algorithm: ENVELOPE_ALGORITHM.to_string(),
        nonce,
        ciphertext: ct,
        wrapped_dek,
    })
}

/// Decrypt a field previously produced by [`encrypt_field`].
pub fn decrypt_field(
    provider: &impl KekProvider,
    field: &EncryptedField,
    key_ref: &KeyRef,
) -> Result<Vec<u8>, CryptoError> {
    if field.algorithm != ENVELOPE_ALGORITHM {
        return Err(CryptoError::InvalidEnvelope);
    }
    if key_ref != &field.key_ref {
        return Err(CryptoError::UnknownKeyRef(key_ref.id.clone()));
    }
    let mut dek = SensitiveKey(provider.unwrap_dek(key_ref, &field.wrapped_dek)?);
    let mut combined = Vec::with_capacity(field.nonce.len() + field.ciphertext.len());
    combined.extend_from_slice(&field.nonce);
    combined.extend_from_slice(&field.ciphertext);
    let plain = aead_decrypt(&dek.0, &combined);
    dek.zeroize();
    plain
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct SensitiveKey([u8; DEK_LEN]);

impl SensitiveKey {
    fn random() -> Self {
        let mut key = [0u8; DEK_LEN];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }
}

/// Encrypt `plaintext` with ChaCha20-Poly1305; returns `nonce || ciphertext+tag`.
fn aead_encrypt(key: &[u8; DEK_LEN], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Encrypt)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn aead_decrypt(key: &[u8; DEK_LEN], nonce_and_ct: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if nonce_and_ct.len() <= NONCE_LEN {
        return Err(CryptoError::Decrypt);
    }
    let (nonce_bytes, ct) = nonce_and_ct.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Decrypt)?;
    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(nonce_bytes);
    let nonce = Nonce::from(nonce_arr);
    cipher.decrypt(&nonce, ct).map_err(|_| CryptoError::Decrypt)
}

fn split_nonce_ciphertext(combined: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if combined.len() <= NONCE_LEN {
        return Err(CryptoError::InvalidEnvelope);
    }
    let (n, c) = combined.split_at(NONCE_LEN);
    Ok((n.to_vec(), c.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_eu_ehds() -> solum_profiles::JurisdictionProfile {
        solum_profiles::load_profile(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml"),
        )
        .expect("eu-ehds.toml")
    }

    fn gate_from(profile: &solum_profiles::JurisdictionProfile) -> FieldCategoryGate<'_> {
        FieldCategoryGate::new(&profile.encryption.required_field_categories)
    }

    #[test]
    fn ferrum_core_links() {
        assert!(ferrum_core_type_name().contains("FerrumError"));
    }

    #[test]
    fn customer_held_accepted() {
        let cfg = KeyManagementConfig {
            custody: KeyCustody::CustomerHeld,
            provider: Some("local-hsm".into()),
        };
        assert!(validate_key_custody(&cfg, &[KeyCustody::CustomerHeld]).is_ok());
    }

    #[test]
    fn operator_held_rejected_when_not_allowed() {
        let cfg = KeyManagementConfig {
            custody: KeyCustody::OperatorHeld,
            provider: None,
        };
        assert!(validate_key_custody(&cfg, &[KeyCustody::CustomerHeld]).is_err());
    }

    #[test]
    fn round_trip_encrypt_decrypt_customer_held() {
        let profile = load_eu_ehds();
        let gate = gate_from(&profile);
        let key_ref = KeyRef::new("hsm/slot-1");
        let mut customer_kek = [0u8; DEK_LEN];
        OsRng.fill_bytes(&mut customer_kek);
        let mut provider = CustomerHeldKekProvider::new();
        provider.register_customer_kek(key_ref.clone(), customer_kek);

        let plain = b"patient-summary-demo";
        let enc = encrypt_field(&gate, &provider, "patient_summary", plain, &key_ref).unwrap();
        assert_eq!(enc.algorithm, ENVELOPE_ALGORITHM);
        assert_eq!(enc.category, "patient_summary");
        let json = serde_json::to_string(&enc).unwrap();
        let decoded: EncryptedField = serde_json::from_str(&json).unwrap();
        let out = decrypt_field(&provider, &decoded, &key_ref).unwrap();
        assert_eq!(out, plain);
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let profile = load_eu_ehds();
        let gate = gate_from(&profile);
        let key_ref = KeyRef::new("hsm/slot-1");
        let mut kek_a = [0u8; DEK_LEN];
        let mut kek_b = [0u8; DEK_LEN];
        OsRng.fill_bytes(&mut kek_a);
        OsRng.fill_bytes(&mut kek_b);

        let mut provider_a = CustomerHeldKekProvider::new();
        provider_a.register_customer_kek(key_ref.clone(), kek_a);
        let enc = encrypt_field(&gate, &provider_a, "clinical_notes", b"secret", &key_ref).unwrap();

        let mut provider_b = CustomerHeldKekProvider::new();
        provider_b.register_customer_kek(key_ref.clone(), kek_b);
        let err = decrypt_field(&provider_b, &enc, &key_ref).expect_err("wrong KEK must fail");
        assert!(matches!(
            err,
            CryptoError::Decrypt | CryptoError::InvalidEnvelope
        ));
    }

    #[test]
    fn category_outside_profile_rejected() {
        let profile = load_eu_ehds();
        let gate = gate_from(&profile);
        assert!(profile
            .encryption
            .required_field_categories
            .iter()
            .all(|c| c != "marketing_segment"));

        let key_ref = KeyRef::new("hsm/slot-1");
        let mut provider = CustomerHeldKekProvider::new();
        provider.register_customer_kek(key_ref.clone(), [7u8; DEK_LEN]);

        let err = encrypt_field(&gate, &provider, "marketing_segment", b"x", &key_ref)
            .expect_err("must reject unknown category");
        assert!(matches!(err, CryptoError::CategoryNotRecognised { .. }));
    }

    #[test]
    fn ephemeral_test_keys_round_trip_with_warning_path() {
        // EphemeralTestKekProvider is documented as never for regulated production.
        let profile = load_eu_ehds();
        let gate = gate_from(&profile);
        let key_ref = KeyRef::new("ephemeral/test-1");
        let mut provider = EphemeralTestKekProvider::new();
        let _ = provider.generate_test_kek(key_ref.clone());

        let enc =
            encrypt_field(&gate, &provider, "consent_record", b"consent-v1", &key_ref).unwrap();
        let out = decrypt_field(&provider, &enc, &key_ref).unwrap();
        assert_eq!(out, b"consent-v1");
    }

    #[test]
    fn validate_field_category_mirrors_consent_style() {
        let profile = load_eu_ehds();
        let gate = gate_from(&profile);
        assert!(validate_field_category(&gate, "laboratory_results").is_ok());
        let err = validate_field_category(&gate, "not-a-category").unwrap_err();
        assert!(matches!(err, CryptoError::CategoryNotRecognised { .. }));
    }
}
