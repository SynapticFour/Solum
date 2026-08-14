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

use crate::{Crypt4ghKeyProvider, CryptoError, HeldKeypair, KeyRef};

/// Re-export for `tests/aws_kms.rs` (`mock_client!(aws_sdk_kms, …)` / SDK types).
/// Gated with this module behind `aws-kms`; not available in default builds.
pub use aws_sdk_kms;
/// Re-export mock harness for the feature-gated integration test.
pub use aws_smithy_mocks;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// On-disk KMS-wrapped Crypt4GH seed (CLI / sidecar layout).
///
/// Produced by `solum crypto wrap-seed` (feature `aws-kms`). Private seed is
/// never stored plaintext; unwrap happens once into process memory.
///
/// `encryption_context` is sent on KMS Encrypt and must match on Decrypt.
/// Empty map = legacy files written before context binding (Decrypt without
/// context). New wraps always populate [`seed_encryption_context`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedSeedFile {
    pub key_ref: String,
    /// KMS key id / alias / ARN used at wrap time (informational for operators;
    /// Decrypt uses the ciphertext blob, not this field, for AWS API calls).
    pub kms_key_id: String,
    pub wrapped_seed: Vec<u8>,
    /// KMS EncryptionContext key/value pairs. Default empty for backward-compat
    /// JSON that omitted this field.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub encryption_context: HashMap<String, String>,
}

/// Default EncryptionContext for Crypt4GH seed wrap/unwrap.
///
/// Binds the ciphertext to Solum's seed purpose and the logical `key_ref` so a
/// blob cannot be decrypted under a mismatched operational identity without
/// KMS rejecting the context.
pub fn seed_encryption_context(key_ref: &str) -> HashMap<String, String> {
    let mut ctx = HashMap::new();
    ctx.insert("solum:purpose".into(), "crypt4gh-seed".into());
    ctx.insert("solum:key_ref".into(), key_ref.to_string());
    ctx
}

impl WrappedSeedFile {
    pub fn load(path: &Path) -> Result<Self, CryptoError> {
        let raw = fs::read_to_string(path).map_err(|e| {
            CryptoError::Provider(format!(
                "failed to read wrapped seed {}: {e}",
                path.display()
            ))
        })?;
        serde_json::from_str(&raw).map_err(|e| {
            CryptoError::Provider(format!(
                "invalid wrapped-seed JSON {}: {e} (expected key_ref, kms_key_id, wrapped_seed)",
                path.display()
            ))
        })
    }

    pub fn write(&self, path: &Path) -> Result<(), CryptoError> {
        let raw = serde_json::to_string_pretty(self)
            .map_err(|e| CryptoError::Provider(format!("serialize wrapped seed: {e}")))?;
        fs::write(path, raw).map_err(|e| {
            CryptoError::Provider(format!("failed to write {}: {e}", path.display()))
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
                CryptoError::Provider(format!("failed to chmod 0600 {}: {e}", path.display()))
            })?;
        }
        Ok(())
    }
}

/// Load every regular file under `dir` as [`WrappedSeedFile`] and unwrap into
/// one [`AwsKmsKeyProvider`]. Fail-closed on empty dir / bad JSON / duplicate refs.
pub async fn load_aws_kms_from_dir(
    kms_client: &KmsClient,
    dir: &Path,
) -> Result<AwsKmsKeyProvider, CryptoError> {
    if !dir.is_dir() {
        return Err(CryptoError::Provider(format!(
            "wrapped-keys-dir is not a directory: {}",
            dir.display()
        )));
    }
    let mut provider = AwsKmsKeyProvider::new();
    let mut loaded = 0usize;
    let mut seen_refs: Vec<String> = Vec::new();

    let entries = fs::read_dir(dir).map_err(|e| {
        CryptoError::Provider(format!(
            "failed to read wrapped-keys-dir {}: {e}",
            dir.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            CryptoError::Provider(format!("failed to read entry in {}: {e}", dir.display()))
        })?;
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| CryptoError::Provider(format!("stat {}: {e}", path.display())))?;
        if !meta.is_file() {
            continue;
        }
        let file = WrappedSeedFile::load(&path)?;
        if seen_refs.iter().any(|r| r == &file.key_ref) {
            return Err(CryptoError::Provider(format!(
                "duplicate key_ref '{}' in wrapped-keys-dir ({})",
                file.key_ref,
                path.display()
            )));
        }
        provider
            .register_wrapped_seed(
                kms_client,
                KeyRef::new(file.key_ref.clone()),
                &file.wrapped_seed,
                &file.encryption_context,
            )
            .await?;
        seen_refs.push(file.key_ref);
        loaded += 1;
    }
    if loaded == 0 {
        return Err(CryptoError::Provider(format!(
            "no wrapped-seed files found in {} (place solum crypto wrap-seed JSON here)",
            dir.display()
        )));
    }
    Ok(provider)
}

/// Build a KMS client from environment variables (no `aws-config` crate).
///
/// Required: `AWS_REGION` or `AWS_DEFAULT_REGION`, `AWS_ACCESS_KEY_ID`,
/// `AWS_SECRET_ACCESS_KEY`. Optional: `AWS_SESSION_TOKEN`.
///
/// Instance-role / IRSA default chains are not loaded here — keeps the
/// dependency tree on Solum's MSRV without `aws-config`.
pub fn client_from_env() -> Result<KmsClient, CryptoError> {
    use aws_sdk_kms::config::{Credentials, Region};
    use std::env;

    let region = env::var("AWS_REGION")
        .or_else(|_| env::var("AWS_DEFAULT_REGION"))
        .map_err(|_| {
            CryptoError::Provider("set AWS_REGION or AWS_DEFAULT_REGION for Solum AWS KMS".into())
        })?;
    let access_key = env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| CryptoError::Provider("set AWS_ACCESS_KEY_ID for Solum AWS KMS".into()))?;
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| CryptoError::Provider("set AWS_SECRET_ACCESS_KEY for Solum AWS KMS".into()))?;
    let session_token = env::var("AWS_SESSION_TOKEN").ok();
    let creds = Credentials::new(access_key, secret_key, session_token, None, "solum-env");
    let conf = aws_sdk_kms::Config::builder()
        .behavior_version_latest()
        .region(Region::new(region))
        .credentials_provider(creds)
        .build();
    Ok(KmsClient::from_conf(conf))
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

    /// Deterministic first registered key (sorted id) for store-at-rest envelopes.
    pub fn first_key_ref(&self) -> Option<KeyRef> {
        self.keys.keys().min().cloned().map(KeyRef::new)
    }

    /// Encrypt a 32-byte Crypt4GH private-key seed under a symmetric KMS key
    /// for durable storage (provisioning helper — not the encrypt/decrypt hot path).
    ///
    /// `encryption_context` is bound into the ciphertext (AWS KMS AAD). Pass
    /// [`seed_encryption_context`] for new wraps; decrypt must use the same map.
    pub async fn wrap_seed(
        kms_client: &KmsClient,
        kms_key_id: &str,
        plaintext_seed: &[u8],
        encryption_context: &HashMap<String, String>,
    ) -> Result<Vec<u8>, CryptoError> {
        let seed = normalize_seed(plaintext_seed)?;
        let mut req = kms_client
            .encrypt()
            .key_id(kms_key_id)
            .plaintext(Blob::new(seed));
        if !encryption_context.is_empty() {
            req = req.set_encryption_context(Some(encryption_context.clone()));
        }
        let out = req
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
    ///
    /// `encryption_context` must match the map used at wrap time (empty only
    /// for legacy blobs that were wrapped without context).
    pub async fn from_wrapped_seed(
        kms_client: &KmsClient,
        key_ref: KeyRef,
        wrapped_seed: &[u8],
        encryption_context: &HashMap<String, String>,
    ) -> Result<Self, CryptoError> {
        let mut provider = Self::new();
        provider
            .register_wrapped_seed(kms_client, key_ref, wrapped_seed, encryption_context)
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
        encryption_context: &HashMap<String, String>,
    ) -> Result<(), CryptoError> {
        if wrapped_seed.is_empty() {
            return Err(CryptoError::Provider(
                "wrapped Crypt4GH seed must be non-empty".into(),
            ));
        }
        let mut req = kms_client
            .decrypt()
            .ciphertext_blob(Blob::new(wrapped_seed.to_vec()));
        if !encryption_context.is_empty() {
            req = req.set_encryption_context(Some(encryption_context.clone()));
        }
        let out = req
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
        Ok(vec![kp.crypt4gh_keys()])
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
