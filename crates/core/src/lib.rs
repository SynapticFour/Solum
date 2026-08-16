// SPDX-License-Identifier: BUSL-1.1
//! Solum product core: wires jurisdiction profiles, crypto posture, audit, and
//! clinical interchange adapters (FHIR first; openEHR staged).
//!
//! This crate holds **Solum-specific** orchestration only. Shared sovereignty
//! primitives come from git-pinned `ferrum-core` via `solum-crypto`. Do not
//! copy Ferrum service logic here.

#![forbid(unsafe_code)]

mod jsonl;
mod migrate;
pub use jsonl::{
    jsonl_max_bytes, rotate_jsonl_if_needed, rotate_jsonl_if_needed_with_max,
    DEFAULT_JSONL_MAX_BYTES,
};
pub use migrate::{
    append_dead_letter, dead_letter_row, extract_fhir_resources, load_fhir_json,
    resource_idempotency_key, MigrateError,
};

use std::path::Path;

use chrono::Utc;
use solum_crypto::{
    Crypt4ghKeyProvider, EncryptedField, FieldCategoryGate, KeyManagementConfig, KeyRef,
};
use solum_profiles::{
    load_profile, validate_startup, validate_transfer, ConsentWorkflow, JurisdictionProfile,
    ProfileError, RuntimeConfig, TransferMechanism,
};
use thiserror::Error;

pub use solum_audit as audit;
pub use solum_consent as consent;
pub use solum_crypto as crypto;
pub use solum_fhir as fhir;
pub use solum_identity as identity;
pub use solum_openehr as openehr;
pub use solum_profiles as profiles;

pub use solum_identity::{ActorFromClaimsError, ActorSource, AuthorizationError, SolumActor};

#[derive(Debug, Error)]
pub enum SolumError {
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error(transparent)]
    Audit(#[from] solum_audit::AuditStoreError),
    #[error(transparent)]
    Consent(#[from] solum_consent::ConsentError),
    #[error(transparent)]
    Crypto(#[from] solum_crypto::CryptoError),
    #[error("consent denied for {subject_id}/{purpose} category {category}: {reason}")]
    ConsentDenied {
        subject_id: String,
        purpose: String,
        category: String,
        reason: String,
    },
    #[error("{0}")]
    Message(String),
}

/// Bootstrap Solum against a jurisdiction profile and refuse to start on mismatch.
pub fn start_with_profile(
    profile_path: impl AsRef<Path>,
    runtime: &RuntimeConfig,
) -> Result<JurisdictionProfile, SolumError> {
    let profile = load_profile(profile_path)?;
    validate_startup(&profile, runtime)?;
    tracing::info!(
        profile = %profile.meta.profile,
        jurisdiction = %profile.meta.jurisdiction,
        "Solum startup validation passed"
    );
    Ok(profile)
}

/// Convenience builder for a conforming EU EHDS test/runtime config.
pub fn example_eu_runtime() -> RuntimeConfig {
    RuntimeConfig {
        storage_region: "EU".into(),
        key_management: KeyManagementConfig {
            custody: solum_crypto::KeyCustody::CustomerHeld,
            provider: Some("customer-hsm-eu".into()),
        },
        enabled_audit_events: vec![
            "access.granted".into(),
            "access.denied".into(),
            "data.read".into(),
            "data.export".into(),
            "data.receive_eehrxf".into(),
            "consent.granted".into(),
            "consent.revoked".into(),
            "identity.authenticated".into(),
            "key.use".into(),
            "residency.transfer_attempt".into(),
        ],
        consent_workflow: ConsentWorkflow::GdprGranular,
        audit_retention_days: 3650,
    }
}

/// Audit retention that sits above every shipped profile floor (kenya-dpa 7300,
/// eu-ehds 3650, dev-local 30). CLI and sidecar use this unless
/// `SOLUM_AUDIT_RETENTION_DAYS` is set.
pub const DEFAULT_RUNTIME_AUDIT_RETENTION_DAYS: u32 = 36500;

/// Apply `SOLUM_STORAGE_REGION` and `SOLUM_AUDIT_RETENTION_DAYS` (or
/// [`DEFAULT_RUNTIME_AUDIT_RETENTION_DAYS`]) onto a runtime built from
/// [`example_eu_runtime`].
pub fn apply_runtime_env_overrides(runtime: &mut RuntimeConfig) {
    if let Ok(region) = std::env::var("SOLUM_STORAGE_REGION") {
        let region = region.trim();
        if !region.is_empty() {
            runtime.storage_region = region.to_string();
        }
    }
    runtime.audit_retention_days = std::env::var("SOLUM_AUDIT_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_RUNTIME_AUDIT_RETENTION_DAYS);
}

/// Pilot profiles (`eu-ehds`, `kenya-dpa`, …) require an explicit
/// `SOLUM_STORAGE_REGION`. The profile's first allowed region is **not**
/// treated as a silent residency claim. `dev-local` (client-asserted caps)
/// skips this so demos work on a laptop.
///
/// This is still operator attestation, not a proof the host is in that
/// region. When `AWS_REGION` / `AWS_DEFAULT_REGION` is set and clearly
/// contradicts an EU/EEA declaration, startup refuses.
pub fn require_operator_region_attestation(
    profile: &JurisdictionProfile,
) -> Result<(), SolumError> {
    if profile.auth.allow_client_asserted_capabilities {
        return Ok(());
    }
    let declared = match std::env::var("SOLUM_STORAGE_REGION") {
        Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return Err(SolumError::Message(format!(
                "pilot profile '{}' requires SOLUM_STORAGE_REGION as an operator residency \
                 attestation (the profile default is not inferred). Example: SOLUM_STORAGE_REGION=EU",
                profile.meta.profile
            )));
        }
    };
    refuse_aws_region_contradiction(&declared)
}

/// If the process has an AWS region env and the operator attested EU/EEA,
/// refuse obvious non-EU AWS regions (us-/ap-/sa-/ca-/me-/af-/il-/mx-).
pub fn refuse_aws_region_contradiction(declared: &str) -> Result<(), SolumError> {
    let aws = std::env::var("AWS_REGION").or_else(|_| std::env::var("AWS_DEFAULT_REGION"));
    let Ok(aws) = aws else {
        return Ok(());
    };
    let aws = aws.trim().to_ascii_lowercase();
    if aws.is_empty() {
        return Ok(());
    }
    let declared_u = declared.trim().to_ascii_uppercase();
    let clearly_non_eu = aws.starts_with("us-")
        || aws.starts_with("ap-")
        || aws.starts_with("sa-")
        || aws.starts_with("ca-")
        || aws.starts_with("me-")
        || aws.starts_with("af-")
        || aws.starts_with("il-")
        || aws.starts_with("mx-");
    if (declared_u == "EU" || declared_u == "EEA") && clearly_non_eu {
        return Err(SolumError::Message(format!(
            "SOLUM_STORAGE_REGION={declared} contradicts AWS_REGION={aws}; \
             EU/EEA attestation cannot run against a non-EU AWS region"
        )));
    }
    Ok(())
}

/// A validated jurisdiction profile bundled with its persistent audit store,
/// consent store, and Crypt4GH key provider.
///
/// Stage-1 callers (CLI today, a future service) should use `Deployment`
/// rather than wiring `solum-audit`, `solum-consent`, and `solum-crypto`
/// separately: every consent decision and field encrypt/decrypt made through
/// `Deployment` also writes the matching audit event in the same call, so
/// the stores cannot silently drift apart under normal use. Direct use of
/// the lower-level crates (e.g. in tests) is still fine when you don't need
/// that guarantee.
pub struct Deployment<P: Crypt4ghKeyProvider> {
    profile: JurisdictionProfile,
    audit: solum_audit::FileAuditStore,
    consent: solum_consent::ConsentStore,
    keys: P,
    /// Optional Ferrum object-storage backend (feature `ferrum-storage-backend`).
    ///
    /// Stored as `Arc<dyn ObjectStorage>` so `Deployment` stays generic only over
    /// the key provider: `ObjectStorage` is object-safe, and callers can pass any
    /// concrete backend (`LocalStorage` today) without a second type parameter.
    #[cfg(feature = "ferrum-storage-backend")]
    storage: Option<std::sync::Arc<dyn ferrum_storage::ObjectStorage>>,
}

impl<P: Crypt4ghKeyProvider> Deployment<P> {
    /// Validate `profile_path` against `runtime` (refusing to start on
    /// mismatch, same as [`start_with_profile`]), then open or create the
    /// audit and consent stores at the given paths and retain `keys` for
    /// Crypt4GH field operations.
    pub fn open(
        profile_path: impl AsRef<Path>,
        runtime: &RuntimeConfig,
        audit_path: impl AsRef<Path>,
        consent_path: impl AsRef<Path>,
        keys: P,
    ) -> Result<Self, SolumError> {
        let profile = start_with_profile(profile_path, runtime)?;
        let audit = solum_audit::FileAuditStore::open(audit_path).map_err(SolumError::Audit)?;
        let consent =
            solum_consent::ConsentStore::open(consent_path).map_err(SolumError::Consent)?;
        Ok(Self {
            profile,
            audit,
            consent,
            keys,
            #[cfg(feature = "ferrum-storage-backend")]
            storage: None,
        })
    }

    /// If `SOLUM_TENANT_ID` is set (non-empty), copy into audit `details.tenant_id`.
    /// Evidence correlation only — not an ACL or multi-tenant router (H5).
    pub fn stamp_tenant_id_into(details: &mut serde_json::Map<String, serde_json::Value>) {
        if let Ok(tid) = std::env::var("SOLUM_TENANT_ID") {
            let tid = tid.trim();
            if !tid.is_empty() {
                details.insert(
                    "tenant_id".into(),
                    serde_json::Value::String(tid.to_string()),
                );
            }
        }
    }

    /// Active jurisdiction profile (validated at [`Self::open`]).
    pub fn profile(&self) -> &JurisdictionProfile {
        &self.profile
    }

    fn append_audit_event(&mut self, mut event: solum_audit::AuditEvent) -> Result<(), SolumError> {
        Self::stamp_tenant_id_into(&mut event.details);
        self.audit.append(event)?;
        Ok(())
    }

    fn string_details(pairs: &[(&str, &str)]) -> serde_json::Map<String, serde_json::Value> {
        let mut details = serde_json::Map::new();
        for (k, v) in pairs {
            details.insert((*k).into(), serde_json::Value::String((*v).to_string()));
        }
        details
    }

    /// Attach a Ferrum [`ferrum_storage::ObjectStorage`] backend (additive).
    ///
    /// Uses `Arc<dyn ObjectStorage>` internally so the concrete backend type
    /// (`LocalStorage`, later S3/OpenDAL) does not become a second generic on
    /// [`Deployment`]. Requires feature `ferrum-storage-backend`.
    #[cfg(feature = "ferrum-storage-backend")]
    pub fn with_storage(mut self, storage: impl ferrum_storage::ObjectStorage + 'static) -> Self {
        self.storage = Some(std::sync::Arc::new(storage));
        self
    }

    /// Encrypt via [`Self::encrypt_field_as`], then persist the serialized
    /// [`EncryptedField`] with `ObjectStorage::put_bytes`.
    #[cfg(feature = "ferrum-storage-backend")]
    pub async fn encrypt_field_and_store(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
        storage_key: &str,
    ) -> Result<EncryptedField, SolumError> {
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| {
                SolumError::Message(
                    "encrypt_field_and_store requires Deployment::with_storage(...)".into(),
                )
            })?
            .clone();
        let field =
            self.encrypt_field_as(category, plaintext, key_ref, actor, subject_id, purpose)?;
        let bytes = serde_json::to_vec(&field)
            .map_err(|e| SolumError::Message(format!("serialize EncryptedField: {e}")))?;
        storage
            .put_bytes(storage_key, &bytes)
            .await
            .map_err(|e| SolumError::Message(format!("storage put_bytes: {e}")))?;
        Ok(field)
    }

    /// Load a serialized [`EncryptedField`] via `ObjectStorage::get`, then decrypt
    /// with [`Self::decrypt_field_as`].
    #[cfg(feature = "ferrum-storage-backend")]
    pub async fn read_and_decrypt_field(
        &mut self,
        storage_key: &str,
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
    ) -> Result<Vec<u8>, SolumError> {
        use tokio::io::AsyncReadExt;

        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| {
                SolumError::Message(
                    "read_and_decrypt_field requires Deployment::with_storage(...)".into(),
                )
            })?
            .clone();
        let mut reader = storage
            .get(storage_key)
            .await
            .map_err(|e| SolumError::Message(format!("storage get: {e}")))?;
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| SolumError::Message(format!("storage read: {e}")))?;
        drop(reader);
        let field: EncryptedField = serde_json::from_slice(&bytes)
            .map_err(|e| SolumError::Message(format!("deserialize EncryptedField: {e}")))?;
        self.decrypt_field_as(&field, key_ref, actor, subject_id, purpose)
    }

    /// Fail-closed capability gate: on miss, write `access.denied` and return
    /// [`SolumError::Authorization`]. On success, write `access.granted`.
    fn authorize_or_deny(
        &mut self,
        actor: &SolumActor,
        capability: &str,
        attempted_operation: &str,
    ) -> Result<(), SolumError> {
        let details = Self::string_details(&[
            ("capability", capability),
            ("attempted_operation", attempted_operation),
        ]);
        if let Err(e) = solum_identity::require_capability(actor, capability) {
            self.append_audit_event(solum_audit::AuditEvent {
                event_type: solum_audit::events::ACCESS_DENIED.into(),
                timestamp: Utc::now(),
                actor: actor.to_audit_string(),
                data_category: None,
                outcome: solum_audit::AuditOutcome::Failure,
                details,
            })?;
            return Err(SolumError::Authorization(e));
        }
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::ACCESS_GRANTED.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: None,
            outcome: solum_audit::AuditOutcome::Success,
            details,
        })?;
        Ok(())
    }

    /// Purpose-level consent (Track B CDR / FHIR). Empty grant scope is enough.
    pub fn require_consent_as(
        &mut self,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
        attempted_operation: &str,
    ) -> Result<(), SolumError> {
        if self.consent.is_granted(subject_id, purpose) {
            return Ok(());
        }
        self.deny_consent(
            actor,
            subject_id,
            purpose,
            "*",
            attempted_operation,
            "active_consent_required",
        )
    }

    /// Fail-closed consent gate for crypto `*_as` methods: require an active
    /// grant for `(subject_id, purpose)` that covers `category` (empty grant
    /// scope = purpose-level; otherwise `category` must appear in scope).
    fn require_consent_for_category(
        &mut self,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
        category: &str,
        attempted_operation: &str,
    ) -> Result<(), SolumError> {
        if self
            .consent
            .is_granted_for_category(subject_id, purpose, category)
        {
            return Ok(());
        }
        let reason = if !self.consent.is_granted(subject_id, purpose) {
            "active_consent_required"
        } else {
            "category_not_in_consent_scope"
        };
        self.deny_consent(
            actor,
            subject_id,
            purpose,
            category,
            attempted_operation,
            reason,
        )
    }

    fn deny_consent(
        &mut self,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
        category: &str,
        attempted_operation: &str,
        reason: &str,
    ) -> Result<(), SolumError> {
        let details = Self::string_details(&[
            ("subject_id", subject_id),
            ("purpose", purpose),
            ("category", category),
            ("attempted_operation", attempted_operation),
            ("reason", reason),
        ]);
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::CONSENT_DENIED.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: Some(category.to_string()),
            outcome: solum_audit::AuditOutcome::Failure,
            details,
        })?;
        Err(SolumError::ConsentDenied {
            subject_id: subject_id.to_string(),
            purpose: purpose.to_string(),
            category: category.to_string(),
            reason: reason.to_string(),
        })
    }

    /// Grant consent for `(subject_id, purpose)` — rejecting purposes the
    /// active profile doesn't recognise — and emit the matching
    /// `consent.granted` audit event in the same call.
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::grant_consent_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    #[deprecated(
        since = "0.1.0",
        note = "use grant_consent_as with SolumActor scopes (capability-checked)"
    )]
    pub(crate) fn grant_consent(
        &mut self,
        subject_id: &str,
        purpose: &str,
        scope: Vec<String>,
        actor: &str,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        solum_consent::validate_purpose(&self.profile, purpose)?;
        let record = self
            .consent
            .grant(subject_id, purpose, scope.clone(), actor)?;
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::CONSENT_GRANTED.into(),
            timestamp: record.recorded_at,
            actor: actor.to_string(),
            data_category: scope.first().cloned(),
            outcome: solum_audit::AuditOutcome::Success,
            details: Self::string_details(&[("subject_id", subject_id), ("purpose", purpose)]),
        })?;
        Ok(record)
    }

    /// [`grant_consent`] with a structured [`SolumActor`] (maps via
    /// [`SolumActor::to_audit_string`]). Requires
    /// [`solum_identity::CAP_CONSENT_GRANT`] in `actor.scopes`.
    pub fn grant_consent_as(
        &mut self,
        subject_id: &str,
        purpose: &str,
        scope: Vec<String>,
        actor: &SolumActor,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CONSENT_GRANT, "grant_consent")?;
        let actor_s = actor.to_audit_string();
        #[allow(deprecated)]
        {
            self.grant_consent(subject_id, purpose, scope, &actor_s)
        }
    }

    /// Revoke consent for `(subject_id, purpose)` (the EEHRxF revocation
    /// right) and emit the matching `consent.revoked` audit event.
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::revoke_consent_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    #[deprecated(
        since = "0.1.0",
        note = "use revoke_consent_as with SolumActor scopes (capability-checked)"
    )]
    pub(crate) fn revoke_consent(
        &mut self,
        subject_id: &str,
        purpose: &str,
        actor: &str,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        let record = self.consent.revoke(subject_id, purpose, actor)?;
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::CONSENT_REVOKED.into(),
            timestamp: record.recorded_at,
            actor: actor.to_string(),
            data_category: None,
            outcome: solum_audit::AuditOutcome::Success,
            details: Self::string_details(&[("subject_id", subject_id), ("purpose", purpose)]),
        })?;
        Ok(record)
    }

    /// [`revoke_consent`] with a structured [`SolumActor`]. Requires
    /// [`solum_identity::CAP_CONSENT_REVOKE`] in `actor.scopes`.
    pub fn revoke_consent_as(
        &mut self,
        subject_id: &str,
        purpose: &str,
        actor: &SolumActor,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CONSENT_REVOKE, "revoke_consent")?;
        let actor_s = actor.to_audit_string();
        #[allow(deprecated)]
        {
            self.revoke_consent(subject_id, purpose, &actor_s)
        }
    }

    /// Encrypt one clinical field category with Crypt4GH and emit a
    /// `data.encrypt` audit event. Unrecognised categories fail **before**
    /// any audit write (same posture as an unrecognised consent purpose).
    /// Crypto failures still write `data.encrypt` with
    /// [`AuditOutcome::Failure`].
    ///
    /// Legacy path — no capability **or** consent check. Callers that need
    /// enforced authorization should use [`Self::encrypt_field_as`]. This
    /// asymmetry is intentional: `*_as` methods carry a [`SolumActor`] with
    /// scopes to check against; plain `&str` actors carry no such information.
    #[deprecated(
        since = "0.1.0",
        note = "use encrypt_field_as (capability + consent gated); &str path bypasses both"
    )]
    #[allow(dead_code)] // exercised by crate tests; product path uses encrypt_field_inner
    pub(crate) fn encrypt_field(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &str,
    ) -> Result<EncryptedField, SolumError> {
        self.encrypt_field_inner(category, plaintext, key_ref, actor, None, None)
    }

    fn encrypt_field_inner(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &str,
        subject_id: Option<&str>,
        purpose: Option<&str>,
    ) -> Result<EncryptedField, SolumError> {
        let gate = FieldCategoryGate::new(&self.profile.encryption.required_field_categories);
        solum_crypto::validate_field_category(&gate, category)?;
        let details = self.crypto_details(category, key_ref, subject_id, purpose);
        match solum_crypto::encrypt_field(&gate, &self.keys, category, plaintext, key_ref) {
            Ok(field) => {
                self.append_crypto_events(
                    actor,
                    category,
                    solum_audit::events::DATA_ENCRYPT,
                    solum_audit::AuditOutcome::Success,
                    details,
                    false,
                )?;
                Ok(field)
            }
            Err(e) => {
                self.append_crypto_events(
                    actor,
                    category,
                    solum_audit::events::DATA_ENCRYPT,
                    solum_audit::AuditOutcome::Failure,
                    details,
                    false,
                )?;
                Err(e.into())
            }
        }
    }

    /// [`encrypt_field`] with a structured [`SolumActor`]. Requires
    /// [`solum_identity::CAP_CRYPTO_ENCRYPT`] in `actor.scopes` **and** an
    /// active consent grant for `(subject_id, purpose)` covering `category`.
    /// Unknown categories fail before the consent check (same posture as the
    /// legacy path's pre-audit rejection).
    pub fn encrypt_field_as(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
    ) -> Result<EncryptedField, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CRYPTO_ENCRYPT, "encrypt_field")?;
        let gate = FieldCategoryGate::new(&self.profile.encryption.required_field_categories);
        solum_crypto::validate_field_category(&gate, category)?;
        self.require_consent_for_category(actor, subject_id, purpose, category, "encrypt_field")?;
        let actor_s = actor.to_audit_string();
        self.encrypt_field_inner(
            category,
            plaintext,
            key_ref,
            &actor_s,
            Some(subject_id),
            Some(purpose),
        )
    }

    /// Decrypt a Crypt4GH field and emit a `data.decrypt` audit event.
    /// Failed attempts (wrong key, tampered ciphertext, …) still write the
    /// event with [`AuditOutcome::Failure`] — a failed access must appear
    /// in the trail, not only successes.
    ///
    /// Legacy path — no capability **or** consent check. Callers that need
    /// enforced authorization should use [`Self::decrypt_field_as`]. This
    /// asymmetry is intentional: `*_as` methods carry a [`SolumActor`] with
    /// scopes to check against; plain `&str` actors carry no such information.
    #[deprecated(
        since = "0.1.0",
        note = "use decrypt_field_as (capability + consent gated); &str path bypasses both"
    )]
    #[allow(dead_code)] // exercised by crate tests; product path uses decrypt_field_inner
    pub(crate) fn decrypt_field(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &str,
    ) -> Result<Vec<u8>, SolumError> {
        self.decrypt_field_inner(field, key_ref, actor, None, None)
    }

    fn decrypt_field_inner(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &str,
        subject_id: Option<&str>,
        purpose: Option<&str>,
    ) -> Result<Vec<u8>, SolumError> {
        let details = self.crypto_details(&field.category, key_ref, subject_id, purpose);
        match solum_crypto::decrypt_field(&self.keys, field, key_ref) {
            Ok(plaintext) => {
                self.append_crypto_events(
                    actor,
                    &field.category,
                    solum_audit::events::DATA_DECRYPT,
                    solum_audit::AuditOutcome::Success,
                    details,
                    true,
                )?;
                Ok(plaintext)
            }
            Err(e) => {
                self.append_crypto_events(
                    actor,
                    &field.category,
                    solum_audit::events::DATA_DECRYPT,
                    solum_audit::AuditOutcome::Failure,
                    details,
                    false,
                )?;
                Err(e.into())
            }
        }
    }

    fn crypto_details(
        &self,
        category: &str,
        key_ref: &KeyRef,
        subject_id: Option<&str>,
        purpose: Option<&str>,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut details = Self::string_details(&[("category", category), ("key_ref", &key_ref.id)]);
        if let Some(s) = subject_id {
            details.insert(
                "subject_id".into(),
                serde_json::Value::String(s.to_string()),
            );
        }
        if let Some(p) = purpose {
            details.insert("purpose".into(), serde_json::Value::String(p.to_string()));
        }
        details
    }

    fn append_crypto_events(
        &mut self,
        actor: &str,
        category: &str,
        primary: &str,
        outcome: solum_audit::AuditOutcome,
        details: serde_json::Map<String, serde_json::Value>,
        also_data_read: bool,
    ) -> Result<(), SolumError> {
        let actor_s = actor.to_string();
        let cat = Some(category.to_string());
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: primary.into(),
            timestamp: Utc::now(),
            actor: actor_s.clone(),
            data_category: cat.clone(),
            outcome,
            details: details.clone(),
        })?;
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::KEY_USE.into(),
            timestamp: Utc::now(),
            actor: actor_s.clone(),
            data_category: cat.clone(),
            outcome,
            details: details.clone(),
        })?;
        if also_data_read {
            self.append_audit_event(solum_audit::AuditEvent {
                event_type: solum_audit::events::DATA_READ.into(),
                timestamp: Utc::now(),
                actor: actor_s,
                data_category: cat,
                outcome,
                details,
            })?;
        }
        Ok(())
    }

    /// [`decrypt_field`] with a structured [`SolumActor`]. Requires
    /// [`solum_identity::CAP_CRYPTO_DECRYPT`] in `actor.scopes` **and** an
    /// active consent grant for `(subject_id, purpose)` covering the field's
    /// category.
    pub fn decrypt_field_as(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
    ) -> Result<Vec<u8>, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CRYPTO_DECRYPT, "decrypt_field")?;
        self.require_consent_for_category(
            actor,
            subject_id,
            purpose,
            &field.category,
            "decrypt_field",
        )?;
        let actor_s = actor.to_audit_string();
        self.decrypt_field_inner(field, key_ref, &actor_s, Some(subject_id), Some(purpose))
    }

    /// Encrypt a typed [`solum_fhir::PatientSummary`] via
    /// [`Self::encrypt_field_as`] (capability + consent + `data.encrypt` audit
    /// on the Deployment [`FileAuditStore`]). Prefer this over
    /// [`solum_fhir::encrypt_patient_summary`], which is crate-local and does
    /// not write durable audit.
    pub fn encrypt_patient_summary_as(
        &mut self,
        summary: &solum_fhir::PatientSummary,
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
    ) -> Result<EncryptedField, SolumError> {
        let plaintext = serde_json::to_vec(summary)
            .map_err(|e| SolumError::Message(format!("serialize PatientSummary: {e}")))?;
        self.encrypt_field_as(
            solum_fhir::PATIENT_SUMMARY_CATEGORY,
            &plaintext,
            key_ref,
            actor,
            subject_id,
            purpose,
        )
    }

    /// Decrypt a typed [`solum_fhir::PatientSummary`] via
    /// [`Self::decrypt_field_as`] (capability + consent + `data.decrypt` audit).
    pub fn decrypt_patient_summary_as(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &SolumActor,
        subject_id: &str,
        purpose: &str,
    ) -> Result<solum_fhir::PatientSummary, SolumError> {
        let plaintext = self.decrypt_field_as(field, key_ref, actor, subject_id, purpose)?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| SolumError::Message(format!("deserialize PatientSummary: {e}")))
    }

    /// Fail-closed gate for Track B CDR writes (`solum:cdr:write`).
    pub fn authorize_cdr_write_as(&mut self, actor: &SolumActor) -> Result<(), SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CDR_WRITE, "cdr_write")
    }

    /// Fail-closed gate for Track B CDR reads (`solum:cdr:read`).
    pub fn authorize_cdr_read_as(&mut self, actor: &SolumActor) -> Result<(), SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CDR_READ, "cdr_read")
    }

    /// Audit a successful CDR façade write after EHRbase accepts the operation.
    pub fn record_cdr_event_as(
        &mut self,
        actor: &SolumActor,
        event_type: &str,
        details: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SolumError> {
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: event_type.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: Some("clinical_cdr".into()),
            outcome: solum_audit::AuditOutcome::Success,
            details,
        })?;
        Ok(())
    }

    /// Whether `subject_id` currently has an active grant for `purpose`.
    pub fn is_consented(&self, subject_id: &str, purpose: &str) -> bool {
        self.consent.is_granted(subject_id, purpose)
    }

    /// Consent grant/revoke history for one subject (in-memory index).
    pub fn consent_history(
        &self,
        subject_id: &str,
    ) -> Result<Vec<solum_consent::ConsentRecord>, SolumError> {
        Ok(self.consent.history_for_subject(subject_id)?)
    }

    /// Validate a cross-border transfer against the active profile and write
    /// `residency.transfer_attempt` (success or failure).
    pub fn check_transfer(
        &mut self,
        mechanism: &TransferMechanism,
        destination: &str,
        actor: &str,
    ) -> Result<(), SolumError> {
        let mut details = Self::string_details(&[("destination", destination)]);
        details.insert(
            "mechanism".into(),
            serde_json::Value::String(format!("{mechanism:?}")),
        );
        match validate_transfer(&self.profile, mechanism, destination) {
            Ok(()) => {
                self.append_audit_event(solum_audit::AuditEvent {
                    event_type: solum_audit::events::RESIDENCY_TRANSFER_ATTEMPT.into(),
                    timestamp: Utc::now(),
                    actor: actor.to_string(),
                    data_category: None,
                    outcome: solum_audit::AuditOutcome::Success,
                    details,
                })?;
                Ok(())
            }
            Err(e) => {
                details.insert("reason".into(), serde_json::Value::String(e.to_string()));
                self.append_audit_event(solum_audit::AuditEvent {
                    event_type: solum_audit::events::RESIDENCY_TRANSFER_ATTEMPT.into(),
                    timestamp: Utc::now(),
                    actor: actor.to_string(),
                    data_category: None,
                    outcome: solum_audit::AuditOutcome::Failure,
                    details,
                })?;
                Err(e.into())
            }
        }
    }

    /// [`check_transfer`] with a structured [`SolumActor`] (`solum:cdr:write`).
    pub fn check_transfer_as(
        &mut self,
        mechanism: &TransferMechanism,
        destination: &str,
        actor: &SolumActor,
    ) -> Result<(), SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CDR_WRITE, "transfer_check")?;
        self.check_transfer(mechanism, destination, &actor.to_audit_string())
    }

    /// Record a successful identity verification (`identity.authenticated`).
    pub fn record_identity_authenticated_as(
        &mut self,
        actor: &SolumActor,
        details: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SolumError> {
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::IDENTITY_AUTHENTICATED.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: None,
            outcome: solum_audit::AuditOutcome::Success,
            details,
        })
    }

    /// Record an audit-log export (`data.export`). Requires
    /// [`solum_identity::CAP_AUDIT_EXPORT`].
    pub fn record_data_export_as(
        &mut self,
        actor: &SolumActor,
        details: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_AUDIT_EXPORT, "audit_export")?;
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::DATA_EXPORT.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: None,
            outcome: solum_audit::AuditOutcome::Success,
            details,
        })
    }

    /// Record inbound EEHRxF / FHIR receipt (`data.receive_eehrxf`).
    pub fn record_data_receive_eehrxf_as(
        &mut self,
        actor: &SolumActor,
        details: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), SolumError> {
        self.append_audit_event(solum_audit::AuditEvent {
            event_type: solum_audit::events::DATA_RECEIVE_EEHRXF.into(),
            timestamp: Utc::now(),
            actor: actor.to_audit_string(),
            data_category: Some("clinical_fhir".into()),
            outcome: solum_audit::AuditOutcome::Success,
            details,
        })
    }

    /// Full audit trail so far (for log review / HELIOS export).
    pub fn audit_events(&self) -> Result<Vec<solum_audit::AuditRecord>, SolumError> {
        Ok(self.audit.read_all()?)
    }

    /// Verify the audit chain has not been tampered with since it was written.
    pub fn verify_audit_chain(&self) -> Result<(), SolumError> {
        Ok(self.audit.verify_chain()?)
    }
}

/// Current consent decision for CLI / read-only callers: `granted`, `revoked`,
/// or `unknown` (no history for that pair).
///
/// # Design decision (`solum consent status` — no `--audit` path)
///
/// [`Deployment::open`] requires an audit path because grant/revoke always
/// co-write audit events. Status is a pure consent-store read and must not
/// force operators to invent a throwaway audit file. This helper therefore
/// validates the profile via [`start_with_profile`], opens
/// [`solum_consent::ConsentStore`] alone, and maps
/// [`solum_consent::ConsentStore::status`] — without constructing a
/// [`Deployment`] or touching the audit log. Mutating consent commands still
/// go through [`Deployment::open`].
pub fn query_consent_status(
    profile_path: impl AsRef<Path>,
    runtime: &RuntimeConfig,
    consent_path: impl AsRef<Path>,
    subject_id: &str,
    purpose: &str,
) -> Result<&'static str, SolumError> {
    let _profile = start_with_profile(profile_path, runtime)?;
    let store = solum_consent::ConsentStore::open(consent_path)?;
    Ok(match store.status(subject_id, purpose) {
        Some(solum_consent::ConsentStatus::Granted) => "granted",
        Some(solum_consent::ConsentStatus::Revoked) => "revoked",
        None => "unknown",
    })
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use solum_crypto::{EphemeralTestKeyProvider, KeyRef};
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Serialise tests that mutate `SOLUM_TENANT_ID` (process-global env).
    static TENANT_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn eu_profile_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
    }

    #[test]
    fn starts_with_conforming_eu_config() {
        let runtime = example_eu_runtime();
        start_with_profile(eu_profile_path(), &runtime).expect("conforming config must start");
    }

    #[test]
    fn eu_mandatory_events_have_product_write_sites() {
        let profile = solum_profiles::load_profile(eu_profile_path()).expect("eu-ehds");
        for event in &profile.audit.mandatory_events {
            assert!(
                solum_audit::events::PRODUCT_EMITTED
                    .iter()
                    .any(|e| e == event),
                "profile mandatory event '{event}' has no product write site"
            );
        }
    }

    #[test]
    fn aborts_on_contradictory_storage_region() {
        let mut runtime = example_eu_runtime();
        runtime.storage_region = "ap-south-1".into();
        let err = start_with_profile(eu_profile_path(), &runtime)
            .expect_err("non-EU storage must refuse start");
        let msg = err.to_string();
        assert!(msg.contains("startup refused") || msg.contains("storage_region"));
    }

    fn open_deployment(dir: &tempfile::TempDir) -> (Deployment<EphemeralTestKeyProvider>, KeyRef) {
        let key_ref = KeyRef::new("ephemeral/test-1");
        let mut keys = EphemeralTestKeyProvider::new();
        keys.generate_test_keypair(key_ref.clone())
            .expect("test keypair");
        let deployment = Deployment::open(
            eu_profile_path(),
            &example_eu_runtime(),
            dir.path().join("audit.jsonl"),
            dir.path().join("consent.jsonl"),
            keys,
        )
        .expect("deployment must open against a conforming profile");
        (deployment, key_ref)
    }

    #[test]
    fn grant_consent_stamps_solum_tenant_id_when_set() {
        let _guard = TENANT_ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        std::env::set_var("SOLUM_TENANT_ID", "tenant-acme-managed-1");
        deployment
            .grant_consent(
                "patient/42",
                "care_provision",
                vec!["patient_summary".into()],
                "practitioner/7",
            )
            .expect("grant");
        std::env::remove_var("SOLUM_TENANT_ID");
        let events = deployment.audit_events().unwrap();
        assert_eq!(
            events[0]
                .event
                .details
                .get("tenant_id")
                .and_then(|v| v.as_str()),
            Some("tenant-acme-managed-1")
        );
    }

    #[test]
    fn grant_consent_writes_matching_audit_event() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);

        deployment
            .grant_consent(
                "patient/42",
                "care_provision",
                vec!["patient_summary".into()],
                "practitioner/7",
            )
            .expect("care_provision is a valid eu-ehds purpose");

        assert!(deployment.is_consented("patient/42", "care_provision"));
        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.event_type, "consent.granted");
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn revoke_consent_writes_matching_audit_event() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);

        deployment
            .grant_consent("patient/42", "care_provision", vec![], "patient/42")
            .unwrap();
        deployment
            .revoke_consent("patient/42", "care_provision", "patient/42")
            .unwrap();

        assert!(!deployment.is_consented("patient/42", "care_provision"));
        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event.event_type, "consent.revoked");
    }

    #[test]
    fn rejects_purpose_not_recognised_by_profile() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);

        let err = deployment
            .grant_consent("patient/42", "marketing", vec![], "practitioner/7")
            .expect_err("marketing is not an eu-ehds required_purpose");
        assert!(err.to_string().contains("marketing"));
        // Rejected purpose must not appear as a consent OR an audit event.
        assert!(!deployment.is_consented("patient/42", "marketing"));
        assert!(deployment.audit_events().unwrap().is_empty());
    }

    #[test]
    fn encrypt_decrypt_round_trip_writes_matching_audit_events() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);

        let plain = b"patient-summary-demo";
        let enc = deployment
            .encrypt_field("patient_summary", plain, &key_ref, "practitioner/7")
            .expect("patient_summary is a required field category");
        let out = deployment
            .decrypt_field(&enc, &key_ref, "practitioner/7")
            .expect("matching key must decrypt");
        assert_eq!(out, plain);

        let events = deployment.audit_events().unwrap();
        let encrypts: Vec<_> = events
            .iter()
            .filter(|r| r.event.event_type == "data.encrypt")
            .collect();
        let decrypts: Vec<_> = events
            .iter()
            .filter(|r| r.event.event_type == "data.decrypt")
            .collect();
        assert_eq!(encrypts.len(), 1);
        assert_eq!(decrypts.len(), 1);
        assert_eq!(
            encrypts[0].event.outcome,
            solum_audit::AuditOutcome::Success
        );
        assert_eq!(
            decrypts[0].event.outcome,
            solum_audit::AuditOutcome::Success
        );
        assert_eq!(
            encrypts[0].event.data_category.as_deref(),
            Some("patient_summary")
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_rejects_unknown_category_without_audit() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);

        let err = deployment
            .encrypt_field("marketing_segment", b"x", &key_ref, "practitioner/7")
            .expect_err("marketing_segment is not a required field category");
        assert!(err.to_string().contains("marketing_segment"));
        assert!(deployment.audit_events().unwrap().is_empty());
    }

    #[test]
    fn decrypt_wrong_key_writes_failure_audit_event() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);

        let enc = deployment
            .encrypt_field("clinical_notes", b"secret", &key_ref, "practitioner/7")
            .unwrap();

        let wrong_ref = KeyRef::new("ephemeral/wrong-slot");
        let err = deployment
            .decrypt_field(&enc, &wrong_ref, "attacker/9")
            .expect_err("wrong key_ref must fail");
        assert!(!err.to_string().is_empty());

        let events = deployment.audit_events().unwrap();
        let decrypts: Vec<_> = events
            .iter()
            .filter(|r| r.event.event_type == solum_audit::events::DATA_DECRYPT)
            .collect();
        assert_eq!(decrypts.len(), 1);
        assert_eq!(
            decrypts[0].event.outcome,
            solum_audit::AuditOutcome::Failure
        );
        assert_eq!(decrypts[0].event.actor, "attacker/9");
        assert_eq!(
            decrypts[0].event.data_category.as_deref(),
            Some("clinical_notes")
        );
        assert!(events
            .iter()
            .any(|r| r.event.event_type == solum_audit::events::KEY_USE));
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn grant_consent_as_ferrum_and_standalone_same_audit_shape() {
        let _guard = TENANT_ENV_LOCK.lock().unwrap();
        std::env::remove_var("SOLUM_TENANT_ID");
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);

        let ferrum_actor = SolumActor {
            subject_id: "researcher@example.org".into(),
            display: None,
            source: ActorSource::FerrumPassport,
            scopes: vec![
                "drs.read".into(),
                "ferrum:analyst".into(),
                identity::CAP_CONSENT_GRANT.into(),
            ],
        };
        let standalone_actor = SolumActor::standalone(
            "practitioner/7",
            vec!["patient/*.read".into(), identity::CAP_CONSENT_GRANT.into()],
        );

        deployment
            .grant_consent_as(
                "patient/100",
                "care_provision",
                vec!["patient_summary".into()],
                &ferrum_actor,
            )
            .unwrap();
        deployment
            .grant_consent_as(
                "patient/101",
                "care_provision",
                vec!["patient_summary".into()],
                &standalone_actor,
            )
            .unwrap();

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 4);
        let granted: Vec<_> = events
            .iter()
            .filter(|r| r.event.event_type == solum_audit::events::CONSENT_GRANTED)
            .collect();
        assert_eq!(granted.len(), 2);
        let a = &granted[0].event;
        let b = &granted[1].event;

        assert_eq!(a.event_type, b.event_type);
        assert_eq!(a.data_category, b.data_category);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(
            a.details.get("purpose").and_then(|v| v.as_str()),
            Some("care_provision")
        );
        assert_eq!(
            b.details.get("purpose").and_then(|v| v.as_str()),
            Some("care_provision")
        );
        assert_eq!(a.actor, "ferrum:passport:researcher@example.org");
        assert_eq!(b.actor, "standalone:practitioner/7");
        assert_ne!(a.actor, b.actor);
        assert!(deployment.verify_audit_chain().is_ok());
    }

    fn assert_authorization_denied(
        events: &[solum_audit::AuditRecord],
        expected_actor: &str,
        capability: &str,
        attempted_operation: &str,
    ) {
        assert_eq!(events.len(), 1);
        let e = &events[0].event;
        assert_eq!(e.event_type, solum_audit::events::ACCESS_DENIED);
        assert_eq!(e.outcome, solum_audit::AuditOutcome::Failure);
        assert_eq!(e.actor, expected_actor);
        assert_eq!(
            e.details.get("capability").and_then(|v| v.as_str()),
            Some(capability)
        );
        assert_eq!(
            e.details
                .get("attempted_operation")
                .and_then(|v| v.as_str()),
            Some(attempted_operation)
        );
    }

    #[test]
    fn grant_consent_as_allowed_with_capability() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        let actor =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CONSENT_GRANT.into()]);

        deployment
            .grant_consent_as(
                "patient/42",
                "care_provision",
                vec!["patient_summary".into()],
                &actor,
            )
            .expect("matching capability must allow grant");

        assert!(deployment.is_consented("patient/42", "care_provision"));
        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].event.event_type,
            solum_audit::events::ACCESS_GRANTED
        );
        assert_eq!(
            events[1].event.event_type,
            solum_audit::events::CONSENT_GRANTED
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn grant_consent_as_denied_without_capability() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        let actor = SolumActor::standalone("practitioner/7", vec!["patient/*.read".into()]);

        let err = deployment
            .grant_consent_as(
                "patient/42",
                "care_provision",
                vec!["patient_summary".into()],
                &actor,
            )
            .expect_err("wrong scopes must deny grant");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert!(!deployment.is_consented("patient/42", "care_provision"));
        assert_authorization_denied(
            &deployment.audit_events().unwrap(),
            "standalone:practitioner/7",
            identity::CAP_CONSENT_GRANT,
            "grant_consent",
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn grant_consent_as_denied_with_empty_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        let actor = SolumActor::standalone("practitioner/7", vec![]);

        let err = deployment
            .grant_consent_as("patient/42", "care_provision", vec![], &actor)
            .expect_err("empty scopes must deny (fail-closed)");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert!(!deployment.is_consented("patient/42", "care_provision"));
        assert_authorization_denied(
            &deployment.audit_events().unwrap(),
            "standalone:practitioner/7",
            identity::CAP_CONSENT_GRANT,
            "grant_consent",
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn revoke_consent_as_allowed_and_denied() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);

        // Seed via legacy &str path (no capability check).
        deployment
            .grant_consent("patient/42", "care_provision", vec![], "setup")
            .unwrap();
        assert!(deployment.is_consented("patient/42", "care_provision"));

        let denied = SolumActor::standalone("patient/42", vec!["unrelated".into()]);
        let err = deployment
            .revoke_consent_as("patient/42", "care_provision", &denied)
            .expect_err("missing revoke capability");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert!(deployment.is_consented("patient/42", "care_provision"));

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 2); // grant + access.denied
        assert_eq!(
            events[1].event.event_type,
            solum_audit::events::ACCESS_DENIED
        );
        assert_eq!(
            events[1]
                .event
                .details
                .get("attempted_operation")
                .and_then(|v| v.as_str()),
            Some("revoke_consent")
        );
        assert!(deployment.verify_audit_chain().is_ok());

        let allowed =
            SolumActor::standalone("patient/42", vec![identity::CAP_CONSENT_REVOKE.into()]);
        deployment
            .revoke_consent_as("patient/42", "care_provision", &allowed)
            .expect("matching capability must allow revoke");
        assert!(!deployment.is_consented("patient/42", "care_provision"));

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 4); // grant + deny + access.granted + consent.revoked
        assert_eq!(
            events[2].event.event_type,
            solum_audit::events::ACCESS_GRANTED
        );
        assert_eq!(
            events[3].event.event_type,
            solum_audit::events::CONSENT_REVOKED
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn revoke_consent_as_denied_with_empty_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        deployment
            .grant_consent("patient/42", "care_provision", vec![], "setup")
            .unwrap();
        let actor = SolumActor::standalone("patient/42", vec![]);

        let err = deployment
            .revoke_consent_as("patient/42", "care_provision", &actor)
            .expect_err("empty scopes must deny revoke");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert!(deployment.is_consented("patient/42", "care_provision"));
        let events = deployment.audit_events().unwrap();
        assert_eq!(
            events[1].event.event_type,
            solum_audit::events::ACCESS_DENIED
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_decrypt_as_denied_then_allowed_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);
        let plain = b"patient-summary-authz";
        let subject = "patient/42";
        let purpose = "care_provision";

        let no_encrypt = SolumActor::standalone("practitioner/7", vec!["patient/*.read".into()]);
        let err = deployment
            .encrypt_field_as(
                "patient_summary",
                plain,
                &key_ref,
                &no_encrypt,
                subject,
                purpose,
            )
            .expect_err("missing encrypt capability");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert_authorization_denied(
            &deployment.audit_events().unwrap(),
            "standalone:practitioner/7",
            identity::CAP_CRYPTO_ENCRYPT,
            "encrypt_field",
        );
        assert!(deployment.verify_audit_chain().is_ok());

        let encrypt_actor =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CRYPTO_ENCRYPT.into()]);
        let err = deployment
            .encrypt_field_as(
                "patient_summary",
                plain,
                &key_ref,
                &encrypt_actor,
                subject,
                purpose,
            )
            .expect_err("encrypt without consent must deny");
        assert!(matches!(err, SolumError::ConsentDenied { .. }));
        assert_eq!(
            deployment
                .audit_events()
                .unwrap()
                .last()
                .unwrap()
                .event
                .event_type,
            "consent.denied"
        );

        deployment
            .grant_consent(subject, purpose, vec!["patient_summary".into()], "setup")
            .unwrap();

        let enc = deployment
            .encrypt_field_as(
                "patient_summary",
                plain,
                &key_ref,
                &encrypt_actor,
                subject,
                purpose,
            )
            .expect("matching encrypt capability + consent");

        let no_decrypt =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CRYPTO_ENCRYPT.into()]);
        let err = deployment
            .decrypt_field_as(&enc, &key_ref, &no_decrypt, subject, purpose)
            .expect_err("encrypt capability must not imply decrypt");
        assert!(matches!(err, SolumError::Authorization(_)));

        let decrypt_actor =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CRYPTO_DECRYPT.into()]);
        let out = deployment
            .decrypt_field_as(&enc, &key_ref, &decrypt_actor, subject, purpose)
            .expect("matching decrypt capability + consent");
        assert_eq!(out, plain);

        deployment
            .revoke_consent(subject, purpose, "setup")
            .unwrap();
        let err = deployment
            .decrypt_field_as(&enc, &key_ref, &decrypt_actor, subject, purpose)
            .expect_err("decrypt after revoke must deny");
        assert!(matches!(err, SolumError::ConsentDenied { .. }));
        assert_eq!(
            deployment
                .audit_events()
                .unwrap()
                .last()
                .unwrap()
                .event
                .event_type,
            "consent.denied"
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_field_as_denied_with_empty_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);
        let actor = SolumActor::standalone("practitioner/7", vec![]);

        let err = deployment
            .encrypt_field_as(
                "patient_summary",
                b"x",
                &key_ref,
                &actor,
                "patient/42",
                "care_provision",
            )
            .expect_err("empty scopes must deny encrypt");
        assert!(matches!(err, SolumError::Authorization(_)));
        assert_authorization_denied(
            &deployment.audit_events().unwrap(),
            "standalone:practitioner/7",
            identity::CAP_CRYPTO_ENCRYPT,
            "encrypt_field",
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn decrypt_field_as_denied_with_empty_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);

        // Seed ciphertext via legacy &str path.
        let enc = deployment
            .encrypt_field("patient_summary", b"secret", &key_ref, "setup")
            .unwrap();
        let actor = SolumActor::standalone("practitioner/7", vec![]);

        let err = deployment
            .decrypt_field_as(&enc, &key_ref, &actor, "patient/42", "care_provision")
            .expect_err("empty scopes must deny decrypt");
        assert!(matches!(err, SolumError::Authorization(_)));
        let events = deployment.audit_events().unwrap();
        assert_eq!(
            events.last().unwrap().event.event_type,
            solum_audit::events::ACCESS_DENIED
        );
        assert_eq!(
            events
                .last()
                .unwrap()
                .event
                .details
                .get("attempted_operation")
                .and_then(|v| v.as_str()),
            Some("decrypt_field")
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_patient_summary_as_writes_audit_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);
        let subject = "patient/42";
        let purpose = "care_provision";
        let actor = SolumActor::standalone(
            "practitioner/7",
            vec![
                identity::CAP_CONSENT_GRANT.into(),
                identity::CAP_CRYPTO_ENCRYPT.into(),
                identity::CAP_CRYPTO_DECRYPT.into(),
            ],
        );
        deployment
            .grant_consent_as(subject, purpose, vec!["patient_summary".into()], &actor)
            .unwrap();

        let summary = solum_fhir::PatientSummary {
            date: "2026-08-11T12:00:00Z".into(),
            author_display: "Nordlicht Praxis".into(),
            patient: solum_fhir::PatientInfo {
                id: "p-42".into(),
                identifier: vec![],
                name: vec![],
                birth_date: None,
            },
            allergies: vec![],
            medications: vec![],
            problems: vec![],
            mii_validation_ref: None,
        };

        let enc = deployment
            .encrypt_patient_summary_as(&summary, &key_ref, &actor, subject, purpose)
            .expect("encrypt_patient_summary_as");
        assert_eq!(enc.category, solum_fhir::PATIENT_SUMMARY_CATEGORY);

        let out = deployment
            .decrypt_patient_summary_as(&enc, &key_ref, &actor, subject, purpose)
            .expect("decrypt_patient_summary_as");
        assert_eq!(out, summary);

        let events = deployment.audit_events().unwrap();
        assert!(events.iter().any(|r| r.event.event_type == "data.encrypt"));
        assert!(events.iter().any(|r| r.event.event_type == "data.decrypt"));
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[cfg(feature = "ferrum-storage-backend")]
    #[tokio::test]
    async fn encrypt_field_and_store_round_trips_via_local_storage() {
        let dir = tempfile::tempdir().unwrap();
        let (deployment, key_ref) = open_deployment(&dir);
        let local = ferrum_storage::LocalStorage::new(dir.path().join("objects"))
            .expect("LocalStorage::new");
        let mut deployment = deployment.with_storage(local);

        let plain = b"patient-summary-storage-demo";
        let storage_key = "fields/patient_summary/demo-1.json";
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
            .unwrap();
        let enc = deployment
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
            .expect("encrypt_field_and_store");
        assert_eq!(enc.category, "patient_summary");

        let out = deployment
            .read_and_decrypt_field(
                storage_key,
                &key_ref,
                &actor,
                "patient/42",
                "care_provision",
            )
            .await
            .expect("read_and_decrypt_field");
        assert_eq!(out, plain);

        let events = deployment.audit_events().unwrap();
        assert!(events.iter().any(|r| r.event.event_type == "data.encrypt"));
        assert!(events.iter().any(|r| r.event.event_type == "data.decrypt"));
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn check_transfer_emits_residency_event() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, _) = open_deployment(&dir);
        deployment
            .check_transfer(&TransferMechanism::HdabMediated, "EU", "practitioner/7")
            .expect("EU HDAB transfer is permitted");
        let err = deployment
            .check_transfer(&TransferMechanism::HdabMediated, "US", "practitioner/7")
            .expect_err("US is not a permitted destination");
        assert!(matches!(err, SolumError::Profile(_)));
        let events = deployment.audit_events().unwrap();
        let attempts: Vec<_> = events
            .iter()
            .filter(|r| r.event.event_type == solum_audit::events::RESIDENCY_TRANSFER_ATTEMPT)
            .collect();
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].event.outcome,
            solum_audit::AuditOutcome::Success
        );
        assert_eq!(
            attempts[1].event.outcome,
            solum_audit::AuditOutcome::Failure
        );
    }
}
