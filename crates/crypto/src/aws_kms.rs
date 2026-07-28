//! AWS KMS envelope for Crypt4GH X25519 seeds (GTM-3).
//!
//! # Why this lives in `solum-crypto` (not a separate crate)
//!
//! [`AwsKmsKeyProvider`] implements [`Crypt4ghKeyProvider`] and shares
//! [`CryptoError`] / [`KeyRef`] with [`CustomerHeldKeyProvider`]. That is the
//! same dependency class as the rest of this crate's key providers — unlike
//! Sprint 5's `solum-auth-verify` split, where JWT/HTTP stacks must not drag
//! into every `solum-identity` consumer (audit/consent). The heavy AWS SDK is
//! still isolated behind the optional `aws-kms` Cargo feature (default off),
//! mirroring `ferrum-storage-backend` on `solum-core`.
//!
//! # Sync/async boundary
//!
//! KMS `Encrypt`/`Decrypt` run only in async constructors / helpers
//! ([`AwsKmsKeyProvider::from_wrapped_seed`], [`AwsKmsKeyProvider::wrap_seed`]).
//! After unwrap, the 32-byte seed is held in memory and the existing
//! synchronous [`Crypt4ghKeyProvider`] trait is implemented without `block_on`.

use std::collections::HashMap;

use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client as KmsClient;
use crypt4gh::keys::get_public_key_from_private_key;
use crypt4gh::Keys;

use crate::{Crypt4ghKeyProvider, CryptoError, KeyRef};

struct HeldKeypair {
    pubkey: Vec<u8>,
    privkey: Vec<u8>,
}

/// Crypt4GH key provider whose private seeds are stored KMS-wrapped at rest
/// and unwrapped once into process memory (async construct, sync trait).
///
/// Does **not** replace [`crate::CustomerHeldKeyProvider`] — that registry of
/// caller-supplied plaintext keypairs remains unchanged for non-AWS deployments.
#[derive(Default)]
pub struct AwsKmsKeyProvider {
    keys: HashMap<String, HeldKeypair>,
}

impl AwsKmsKeyProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Encrypt a 32-byte Crypt4GH private-key seed under a symmetric KMS key
    /// for durable storage (provisioning helper — not the encrypt/decrypt hot path).
    pub async fn wrap_seed(
        kms_client: &KmsClient,
        kms_key_id: &str,
        plaintext_seed: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let seed = normalize_seed(plaintext_seed)?;
        let out = kms_client
            .encrypt()
            .key_id(kms_key_id)
            .plaintext(Blob::new(seed))
            .send()
            .await
            .map_err(|e| CryptoError::Provider(format!("KMS Encrypt failed: {e}")))?;
        let blob = out.ciphertext_blob.ok_or_else(|| {
            CryptoError::Provider("KMS Encrypt returned no ciphertext_blob".into())
        })?;
        Ok(blob.into_inner())
    }

    /// Decrypt a KMS-wrapped Crypt4GH seed and retain it for subsequent sync
    /// [`Crypt4ghKeyProvider`] calls. Public key is derived from the seed.
    pub async fn from_wrapped_seed(
        kms_client: &KmsClient,
        key_ref: KeyRef,
        wrapped_seed: &[u8],
    ) -> Result<Self, CryptoError> {
        let mut provider = Self::new();
        provider
            .register_wrapped_seed(kms_client, key_ref, wrapped_seed)
            .await?;
        Ok(provider)
    }

    /// Async unwrap + register additional key (same semantics as
    /// [`from_wrapped_seed`] for a single entry).
    pub async fn register_wrapped_seed(
        &mut self,
        kms_client: &KmsClient,
        key_ref: KeyRef,
        wrapped_seed: &[u8],
    ) -> Result<(), CryptoError> {
        if wrapped_seed.is_empty() {
            return Err(CryptoError::Provider(
                "wrapped Crypt4GH seed must be non-empty".into(),
            ));
        }
        let out = kms_client
            .decrypt()
            .ciphertext_blob(Blob::new(wrapped_seed.to_vec()))
            .send()
            .await
            .map_err(|e| CryptoError::Provider(format!("KMS Decrypt failed: {e}")))?;
        let plaintext = out
            .plaintext
            .ok_or_else(|| CryptoError::Provider("KMS Decrypt returned no plaintext".into()))?;
        let privkey = normalize_seed(plaintext.as_ref())?;
        let pubkey = get_public_key_from_private_key(&privkey)
            .map_err(|e| CryptoError::Provider(e.to_string()))?;
        self.keys.insert(
            key_ref.id,
            HeldKeypair {
                pubkey,
                privkey: privkey.to_vec(),
            },
        );
        Ok(())
    }
}

impl Crypt4ghKeyProvider for AwsKmsKeyProvider {
    fn recipient_pubkey(&self, key_ref: &KeyRef) -> Result<Vec<u8>, CryptoError> {
        self.keys
            .get(&key_ref.id)
            .map(|k| k.pubkey.clone())
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))
    }

    fn private_keys(&self, key_ref: &KeyRef) -> Result<Vec<Keys>, CryptoError> {
        let kp = self
            .keys
            .get(&key_ref.id)
            .ok_or_else(|| CryptoError::UnknownKeyRef(key_ref.id.clone()))?;
        Ok(vec![Keys {
            method: 0,
            privkey: kp.privkey.clone(),
            recipient_pubkey: kp.pubkey.clone(),
        }])
    }
}

fn normalize_seed(plaintext_seed: &[u8]) -> Result<[u8; 32], CryptoError> {
    if plaintext_seed.len() < 32 {
        return Err(CryptoError::Provider(
            "Crypt4GH private seed must be >= 32 bytes".into(),
        ));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&plaintext_seed[..32]);
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decrypt_field, encrypt_field, CustomerHeldKeyProvider, FieldCategoryGate};
    use aws_sdk_kms::operation::decrypt::DecryptOutput;
    use aws_sdk_kms::operation::encrypt::EncryptOutput;
    use aws_sdk_kms::Client;
    use aws_smithy_mocks::{mock, mock_client, RuleMode};
    use crypt4gh::keys::generate_private_key;

    fn categories() -> Vec<String> {
        vec!["patient_summary".into()]
    }

    #[tokio::test]
    async fn wrap_then_from_wrapped_matches_customer_held_round_trip() {
        let privkey = generate_private_key();
        let seed = privkey[..32].to_vec();
        let key_ref = KeyRef::new("kms/slot-1");
        let kms_key_id = "arn:aws:kms:eu-central-1:111122223333:key/test";
        let wrapped_marker = b"mock-kms-ciphertext-for-crypt4gh-seed".to_vec();

        let encrypt_rule = mock!(Client::encrypt)
            .match_requests({
                let seed = seed.clone();
                let kms_key_id = kms_key_id.to_string();
                move |req| {
                    req.key_id() == Some(kms_key_id.as_str())
                        && req.plaintext().map(|b| b.as_ref()) == Some(seed.as_slice())
                }
            })
            .then_output({
                let wrapped = wrapped_marker.clone();
                let kms_key_id = kms_key_id.to_string();
                move || {
                    EncryptOutput::builder()
                        .ciphertext_blob(Blob::new(wrapped.clone()))
                        .key_id(kms_key_id.clone())
                        .build()
                }
            });

        let decrypt_rule = mock!(Client::decrypt)
            .match_requests({
                let wrapped = wrapped_marker.clone();
                move |req| req.ciphertext_blob().map(|b| b.as_ref()) == Some(wrapped.as_slice())
            })
            .then_output({
                let seed = seed.clone();
                let kms_key_id = kms_key_id.to_string();
                move || {
                    DecryptOutput::builder()
                        .plaintext(Blob::new(seed.clone()))
                        .key_id(kms_key_id.clone())
                        .build()
                }
            });

        let client = mock_client!(
            aws_sdk_kms,
            RuleMode::MatchAny,
            [&encrypt_rule, &decrypt_rule]
        );

        let wrapped = AwsKmsKeyProvider::wrap_seed(&client, kms_key_id, &seed)
            .await
            .expect("wrap_seed");
        assert_eq!(wrapped, wrapped_marker);
        assert_eq!(encrypt_rule.num_calls(), 1);

        let kms_provider = AwsKmsKeyProvider::from_wrapped_seed(&client, key_ref.clone(), &wrapped)
            .await
            .expect("from_wrapped_seed");
        assert_eq!(decrypt_rule.num_calls(), 1);

        let mut customer = CustomerHeldKeyProvider::new();
        let pubkey = get_public_key_from_private_key(&seed).unwrap();
        customer
            .register_customer_keypair(key_ref.clone(), pubkey, seed.clone())
            .unwrap();

        let cats = categories();
        let gate = FieldCategoryGate::new(&cats);
        let plain = b"patient-summary-kms-demo";
        let enc_kms = encrypt_field(&gate, &kms_provider, "patient_summary", plain, &key_ref)
            .expect("kms encrypt");
        let out_kms = decrypt_field(&kms_provider, &enc_kms, &key_ref).expect("kms decrypt");
        assert_eq!(out_kms, plain);

        let enc_cust = encrypt_field(&gate, &customer, "patient_summary", plain, &key_ref)
            .expect("customer encrypt");
        let out_cust = decrypt_field(&customer, &enc_cust, &key_ref).expect("customer decrypt");
        assert_eq!(out_cust, plain);

        // Same seed material: ciphertext for the customer pubkey decrypts via KMS provider.
        let cross = decrypt_field(&kms_provider, &enc_cust, &key_ref).expect("cross decrypt");
        assert_eq!(cross, plain);
    }

    #[tokio::test]
    async fn wrap_seed_rejects_short_seed() {
        let encrypt_rule = mock!(Client::encrypt).then_output(|| {
            EncryptOutput::builder()
                .ciphertext_blob(Blob::new(b"x".as_slice()))
                .build()
        });
        let client = mock_client!(aws_sdk_kms, [&encrypt_rule]);
        let err = AwsKmsKeyProvider::wrap_seed(&client, "alias/test", &[1u8; 16])
            .await
            .expect_err("short seed");
        assert!(matches!(err, CryptoError::Provider(_)));
        assert_eq!(encrypt_rule.num_calls(), 0);
    }
}
