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
pub use solum_openehr as openehr;
pub use solum_profiles as profiles;

#[derive(Debug, Error)]
pub enum SolumError {
    #[error(transparent)]
    Profile(#[from] ProfileError),
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
        })
    }

    /// Grant consent for `(subject_id, purpose)` — rejecting purposes the
    /// active profile doesn't recognise — and emit the matching
    /// `consent.granted` audit event in the same call.
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

    /// Revoke consent for `(subject_id, purpose)` (the EEHRxF revocation
    /// right) and emit the matching `consent.revoked` audit event.
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

    /// Encrypt one clinical field category with Crypt4GH and emit a
    /// `data.encrypt` audit event. Unrecognised categories fail **before**
    /// any audit write (same posture as an unrecognised consent purpose).
    /// Crypto failures still write `data.encrypt` with
    /// [`AuditOutcome::Failure`].
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

    /// Decrypt a Crypt4GH field and emit a `data.decrypt` audit event.
    /// Failed attempts (wrong key, tampered ciphertext, …) still write the
    /// event with [`AuditOutcome::Failure`] — a failed access must appear
    /// in the trail, not only successes.
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
}
