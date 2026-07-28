//! Solum product core: wires jurisdiction profiles, crypto posture, audit, and
//! clinical interchange adapters (FHIR first; openEHR staged).
//!
//! This crate holds **Solum-specific** orchestration only. Shared sovereignty
//! primitives come from git-pinned `ferrum-core` via `solum-crypto`. Do not
//! copy Ferrum service logic here.

#![forbid(unsafe_code)]

use std::path::Path;

use chrono::Utc;
use solum_crypto::{
    Crypt4ghKeyProvider, EncryptedField, FieldCategoryGate, KeyManagementConfig, KeyRef,
};
use solum_profiles::{
    load_profile, validate_startup, ConsentWorkflow, JurisdictionProfile, ProfileError,
    RuntimeConfig,
};
use thiserror::Error;

pub use solum_audit as audit;
pub use solum_consent as consent;
pub use solum_crypto as crypto;
pub use solum_fhir as fhir;
pub use solum_identity as identity;
pub use solum_openehr as openehr;
pub use solum_profiles as profiles;

pub use solum_identity::{ActorSource, AuthorizationError, SolumActor};

#[derive(Debug, Error)]
pub enum SolumError {
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
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
    }
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
    pub profile: JurisdictionProfile,
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
        let audit = solum_audit::FileAuditStore::open(audit_path)
            .map_err(|e| SolumError::Message(format!("audit store: {e}")))?;
        let consent = solum_consent::ConsentStore::open(consent_path)
            .map_err(|e| SolumError::Message(format!("consent store: {e}")))?;
        Ok(Self {
            profile,
            audit,
            consent,
            keys,
            #[cfg(feature = "ferrum-storage-backend")]
            storage: None,
        })
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

    /// Encrypt via the existing sync [`Self::encrypt_field`], then persist the
    /// serialized [`EncryptedField`] with `ObjectStorage::put_bytes`.
    ///
    /// Async only at the storage boundary — no `block_on` inside this crate.
    /// Requires feature `ferrum-storage-backend` and a prior [`Self::with_storage`].
    #[cfg(feature = "ferrum-storage-backend")]
    pub async fn encrypt_field_and_store(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &str,
        storage_key: &str,
    ) -> Result<EncryptedField, SolumError> {
        // Clone the Arc so encrypt_field can take &mut self without overlapping borrows.
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| {
                SolumError::Message(
                    "encrypt_field_and_store requires Deployment::with_storage(...)".into(),
                )
            })?
            .clone();
        let field = self.encrypt_field(category, plaintext, key_ref, actor)?;
        let bytes = serde_json::to_vec(&field)
            .map_err(|e| SolumError::Message(format!("serialize EncryptedField: {e}")))?;
        storage
            .put_bytes(storage_key, &bytes)
            .await
            .map_err(|e| SolumError::Message(format!("storage put_bytes: {e}")))?;
        Ok(field)
    }

    /// Load a serialized [`EncryptedField`] via `ObjectStorage::get`, then decrypt
    /// with the existing sync [`Self::decrypt_field`].
    ///
    /// Takes `&mut self` because [`Self::decrypt_field`] co-writes the audit event
    /// (same as the non-storage path). Requires feature `ferrum-storage-backend`.
    #[cfg(feature = "ferrum-storage-backend")]
    pub async fn read_and_decrypt_field(
        &mut self,
        storage_key: &str,
        key_ref: &KeyRef,
        actor: &str,
    ) -> Result<Vec<u8>, SolumError> {
        use tokio::io::AsyncReadExt;

        // Clone the Arc; drop the reader before decrypt_field (&mut self + audit).
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
        self.decrypt_field(&field, key_ref, actor)
    }

    /// Fail-closed capability gate for `*_as` methods: on miss, write one
    /// `authorization.denied` audit event and return [`SolumError::Authorization`].
    fn authorize_or_deny(
        &mut self,
        actor: &SolumActor,
        capability: &str,
        attempted_operation: &str,
    ) -> Result<(), SolumError> {
        if let Err(e) = solum_identity::require_capability(actor, capability) {
            let mut details = serde_json::Map::new();
            details.insert(
                "capability".into(),
                serde_json::Value::String(capability.to_string()),
            );
            details.insert(
                "attempted_operation".into(),
                serde_json::Value::String(attempted_operation.to_string()),
            );
            self.audit
                .append(solum_audit::AuditEvent {
                    event_type: "authorization.denied".into(),
                    timestamp: Utc::now(),
                    actor: actor.to_audit_string(),
                    data_category: None,
                    outcome: solum_audit::AuditOutcome::Failure,
                    details,
                })
                .map_err(|audit_err| SolumError::Message(audit_err.to_string()))?;
            return Err(SolumError::Authorization(e));
        }
        Ok(())
    }

    /// Grant consent for `(subject_id, purpose)` — rejecting purposes the
    /// active profile doesn't recognise — and emit the matching
    /// `consent.granted` audit event in the same call.
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::grant_consent_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    pub fn grant_consent(
        &mut self,
        subject_id: &str,
        purpose: &str,
        scope: Vec<String>,
        actor: &str,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        solum_consent::validate_purpose(&self.profile, purpose)
            .map_err(|e| SolumError::Message(e.to_string()))?;
        let record = self
            .consent
            .grant(subject_id, purpose, scope.clone(), actor)
            .map_err(|e| SolumError::Message(e.to_string()))?;
        self.audit
            .append(solum_audit::AuditEvent {
                event_type: "consent.granted".into(),
                timestamp: record.recorded_at,
                actor: actor.to_string(),
                data_category: scope.first().cloned(),
                outcome: solum_audit::AuditOutcome::Success,
                details: Default::default(),
            })
            .map_err(|e| SolumError::Message(e.to_string()))?;
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
        self.grant_consent(subject_id, purpose, scope, &actor_s)
    }

    /// Revoke consent for `(subject_id, purpose)` (the EEHRxF revocation
    /// right) and emit the matching `consent.revoked` audit event.
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::revoke_consent_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    pub fn revoke_consent(
        &mut self,
        subject_id: &str,
        purpose: &str,
        actor: &str,
    ) -> Result<solum_consent::ConsentRecord, SolumError> {
        let record = self
            .consent
            .revoke(subject_id, purpose, actor)
            .map_err(|e| SolumError::Message(e.to_string()))?;
        self.audit
            .append(solum_audit::AuditEvent {
                event_type: "consent.revoked".into(),
                timestamp: record.recorded_at,
                actor: actor.to_string(),
                data_category: None,
                outcome: solum_audit::AuditOutcome::Success,
                details: Default::default(),
            })
            .map_err(|e| SolumError::Message(e.to_string()))?;
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
        self.revoke_consent(subject_id, purpose, &actor_s)
    }

    /// Encrypt one clinical field category with Crypt4GH and emit a
    /// `data.encrypt` audit event. Unrecognised categories fail **before**
    /// any audit write (same posture as an unrecognised consent purpose).
    /// Crypto failures still write `data.encrypt` with
    /// [`AuditOutcome::Failure`].
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::encrypt_field_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    pub fn encrypt_field(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &str,
    ) -> Result<EncryptedField, SolumError> {
        let gate = FieldCategoryGate::new(&self.profile.encryption.required_field_categories);
        // Reject unknown categories without an audit event (marketing-purpose pattern).
        solum_crypto::validate_field_category(&gate, category)
            .map_err(|e| SolumError::Message(e.to_string()))?;

        match solum_crypto::encrypt_field(&gate, &self.keys, category, plaintext, key_ref) {
            Ok(field) => {
                self.audit
                    .append(solum_audit::AuditEvent {
                        event_type: "data.encrypt".into(),
                        timestamp: Utc::now(),
                        actor: actor.to_string(),
                        data_category: Some(category.to_string()),
                        outcome: solum_audit::AuditOutcome::Success,
                        details: Default::default(),
                    })
                    .map_err(|e| SolumError::Message(e.to_string()))?;
                Ok(field)
            }
            Err(e) => {
                self.audit
                    .append(solum_audit::AuditEvent {
                        event_type: "data.encrypt".into(),
                        timestamp: Utc::now(),
                        actor: actor.to_string(),
                        data_category: Some(category.to_string()),
                        outcome: solum_audit::AuditOutcome::Failure,
                        details: Default::default(),
                    })
                    .map_err(|audit_err| SolumError::Message(audit_err.to_string()))?;
                Err(SolumError::Message(e.to_string()))
            }
        }
    }

    /// [`encrypt_field`] with a structured [`SolumActor`]. Requires
    /// [`solum_identity::CAP_CRYPTO_ENCRYPT`] in `actor.scopes`.
    pub fn encrypt_field_as(
        &mut self,
        category: &str,
        plaintext: &[u8],
        key_ref: &KeyRef,
        actor: &SolumActor,
    ) -> Result<EncryptedField, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CRYPTO_ENCRYPT, "encrypt_field")?;
        let actor_s = actor.to_audit_string();
        self.encrypt_field(category, plaintext, key_ref, &actor_s)
    }

    /// Decrypt a Crypt4GH field and emit a `data.decrypt` audit event.
    /// Failed attempts (wrong key, tampered ciphertext, …) still write the
    /// event with [`AuditOutcome::Failure`] — a failed access must appear
    /// in the trail, not only successes.
    ///
    /// Legacy path — no capability check. Callers that need enforced
    /// authorization should use [`Self::decrypt_field_as`]. This asymmetry is
    /// intentional: `*_as` methods carry a [`SolumActor`] with scopes to check
    /// against; plain `&str` actors carry no such information.
    pub fn decrypt_field(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &str,
    ) -> Result<Vec<u8>, SolumError> {
        match solum_crypto::decrypt_field(&self.keys, field, key_ref) {
            Ok(plaintext) => {
                self.audit
                    .append(solum_audit::AuditEvent {
                        event_type: "data.decrypt".into(),
                        timestamp: Utc::now(),
                        actor: actor.to_string(),
                        data_category: Some(field.category.clone()),
                        outcome: solum_audit::AuditOutcome::Success,
                        details: Default::default(),
                    })
                    .map_err(|e| SolumError::Message(e.to_string()))?;
                Ok(plaintext)
            }
            Err(e) => {
                self.audit
                    .append(solum_audit::AuditEvent {
                        event_type: "data.decrypt".into(),
                        timestamp: Utc::now(),
                        actor: actor.to_string(),
                        data_category: Some(field.category.clone()),
                        outcome: solum_audit::AuditOutcome::Failure,
                        details: Default::default(),
                    })
                    .map_err(|audit_err| SolumError::Message(audit_err.to_string()))?;
                Err(SolumError::Message(e.to_string()))
            }
        }
    }

    /// [`decrypt_field`] with a structured [`SolumActor`]. Requires
    /// [`solum_identity::CAP_CRYPTO_DECRYPT`] in `actor.scopes`.
    pub fn decrypt_field_as(
        &mut self,
        field: &EncryptedField,
        key_ref: &KeyRef,
        actor: &SolumActor,
    ) -> Result<Vec<u8>, SolumError> {
        self.authorize_or_deny(actor, solum_identity::CAP_CRYPTO_DECRYPT, "decrypt_field")?;
        let actor_s = actor.to_audit_string();
        self.decrypt_field(field, key_ref, &actor_s)
    }

    /// Whether `subject_id` currently has an active grant for `purpose`.
    pub fn is_consented(&self, subject_id: &str, purpose: &str) -> bool {
        self.consent.is_granted(subject_id, purpose)
    }

    /// Full audit trail so far (for log review / HELIOS export).
    pub fn audit_events(&self) -> Result<Vec<solum_audit::AuditRecord>, SolumError> {
        self.audit
            .read_all()
            .map_err(|e| SolumError::Message(e.to_string()))
    }

    /// Verify the audit chain has not been tampered with since it was written.
    pub fn verify_audit_chain(&self) -> Result<(), SolumError> {
        self.audit
            .verify_chain()
            .map_err(|e| SolumError::Message(e.to_string()))
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
    let store = solum_consent::ConsentStore::open(consent_path)
        .map_err(|e| SolumError::Message(format!("consent store: {e}")))?;
    Ok(match store.status(subject_id, purpose) {
        Some(solum_consent::ConsentStatus::Granted) => "granted",
        Some(solum_consent::ConsentStatus::Revoked) => "revoked",
        None => "unknown",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solum_crypto::{EphemeralTestKeyProvider, KeyRef};
    use std::path::PathBuf;

    fn eu_profile_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles/eu-ehds.toml")
    }

    #[test]
    fn starts_with_conforming_eu_config() {
        let runtime = example_eu_runtime();
        start_with_profile(eu_profile_path(), &runtime).expect("conforming config must start");
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
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.event_type, "data.encrypt");
        assert_eq!(events[0].event.outcome, solum_audit::AuditOutcome::Success);
        assert_eq!(events[1].event.event_type, "data.decrypt");
        assert_eq!(events[1].event.outcome, solum_audit::AuditOutcome::Failure);
        assert_eq!(events[1].event.actor, "attacker/9");
        assert_eq!(
            events[1].event.data_category.as_deref(),
            Some("clinical_notes")
        );
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn grant_consent_as_ferrum_and_standalone_same_audit_shape() {
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
        assert_eq!(events.len(), 2);
        let a = &events[0].event;
        let b = &events[1].event;

        assert_eq!(a.event_type, b.event_type);
        assert_eq!(a.data_category, b.data_category);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.details, b.details);
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
        assert_eq!(e.event_type, "authorization.denied");
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
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.event_type, "consent.granted");
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
        assert_eq!(events.len(), 2); // grant + authorization.denied
        assert_eq!(events[1].event.event_type, "authorization.denied");
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
        assert_eq!(events.len(), 3);
        assert_eq!(events[2].event.event_type, "consent.revoked");
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
        assert_eq!(events[1].event.event_type, "authorization.denied");
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_decrypt_as_denied_then_allowed_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);
        let plain = b"patient-summary-authz";

        let no_encrypt = SolumActor::standalone("practitioner/7", vec!["patient/*.read".into()]);
        let err = deployment
            .encrypt_field_as("patient_summary", plain, &key_ref, &no_encrypt)
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
        let enc = deployment
            .encrypt_field_as("patient_summary", plain, &key_ref, &encrypt_actor)
            .expect("matching encrypt capability");

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 2); // denied + data.encrypt
        assert_eq!(events[1].event.event_type, "data.encrypt");
        assert!(deployment.verify_audit_chain().is_ok());

        let no_decrypt =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CRYPTO_ENCRYPT.into()]);
        let err = deployment
            .decrypt_field_as(&enc, &key_ref, &no_decrypt)
            .expect_err("encrypt capability must not imply decrypt");
        assert!(matches!(err, SolumError::Authorization(_)));

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[2].event.event_type, "authorization.denied");
        assert_eq!(
            events[2]
                .event
                .details
                .get("capability")
                .and_then(|v| v.as_str()),
            Some(identity::CAP_CRYPTO_DECRYPT)
        );
        assert!(deployment.verify_audit_chain().is_ok());

        let decrypt_actor =
            SolumActor::standalone("practitioner/7", vec![identity::CAP_CRYPTO_DECRYPT.into()]);
        let out = deployment
            .decrypt_field_as(&enc, &key_ref, &decrypt_actor)
            .expect("matching decrypt capability");
        assert_eq!(out, plain);

        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[3].event.event_type, "data.decrypt");
        assert_eq!(events[3].event.outcome, solum_audit::AuditOutcome::Success);
        assert!(deployment.verify_audit_chain().is_ok());
    }

    #[test]
    fn encrypt_field_as_denied_with_empty_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut deployment, key_ref) = open_deployment(&dir);
        let actor = SolumActor::standalone("practitioner/7", vec![]);

        let err = deployment
            .encrypt_field_as("patient_summary", b"x", &key_ref, &actor)
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
            .decrypt_field_as(&enc, &key_ref, &actor)
            .expect_err("empty scopes must deny decrypt");
        assert!(matches!(err, SolumError::Authorization(_)));
        let events = deployment.audit_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event.event_type, "authorization.denied");
        assert_eq!(
            events[1]
                .event
                .details
                .get("attempted_operation")
                .and_then(|v| v.as_str()),
            Some("decrypt_field")
        );
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
        let enc = deployment
            .encrypt_field_and_store(
                "patient_summary",
                plain,
                &key_ref,
                "practitioner/7",
                storage_key,
            )
            .await
            .expect("encrypt_field_and_store");
        assert_eq!(enc.category, "patient_summary");

        let out = deployment
            .read_and_decrypt_field(storage_key, &key_ref, "practitioner/7")
            .await
            .expect("read_and_decrypt_field");
        assert_eq!(out, plain);

        let events = deployment.audit_events().unwrap();
        assert!(events.iter().any(|r| r.event.event_type == "data.encrypt"));
        assert!(events.iter().any(|r| r.event.event_type == "data.decrypt"));
        assert!(deployment.verify_audit_chain().is_ok());
    }
}
