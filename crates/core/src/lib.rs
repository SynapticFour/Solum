//! Solum product core: wires jurisdiction profiles, crypto posture, audit, and
//! clinical interchange adapters (FHIR first; openEHR staged).
//!
//! This crate holds **Solum-specific** orchestration only. Shared sovereignty
//! primitives come from git-pinned `ferrum-core` via `solum-crypto`. Do not
//! copy Ferrum service logic here.

#![forbid(unsafe_code)]

use std::path::Path;

use solum_crypto::KeyManagementConfig;
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

/// A validated jurisdiction profile bundled with its persistent audit and
/// consent stores.
///
/// Stage-1 callers (CLI today, a future service) should use `Deployment`
/// rather than wiring `solum-audit` and `solum-consent` separately: every
/// consent decision made through `Deployment` also writes the matching
/// `consent.granted` / `consent.revoked` audit event in the same call, so
/// the two stores cannot silently drift apart under normal use. Direct use
/// of `solum_consent::ConsentStore` (e.g. in tests) is still fine when you
/// don't need that guarantee.
pub struct Deployment {
    pub profile: JurisdictionProfile,
    audit: solum_audit::FileAuditStore,
    consent: solum_consent::ConsentStore,
}

impl Deployment {
    /// Validate `profile_path` against `runtime` (refusing to start on
    /// mismatch, same as [`start_with_profile`]), then open or create the
    /// audit and consent stores at the given paths.
    pub fn open(
        profile_path: impl AsRef<Path>,
        runtime: &RuntimeConfig,
        audit_path: impl AsRef<Path>,
        consent_path: impl AsRef<Path>,
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

    fn open_deployment(dir: &tempfile::TempDir) -> Deployment {
        Deployment::open(
            eu_profile_path(),
            &example_eu_runtime(),
            dir.path().join("audit.jsonl"),
            dir.path().join("consent.jsonl"),
        )
        .expect("deployment must open against a conforming profile")
    }

    #[test]
    fn grant_consent_writes_matching_audit_event() {
        let dir = tempfile::tempdir().unwrap();
        let mut deployment = open_deployment(&dir);

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
        let mut deployment = open_deployment(&dir);

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
        let mut deployment = open_deployment(&dir);

        let err = deployment
            .grant_consent("patient/42", "marketing", vec![], "practitioner/7")
            .expect_err("marketing is not an eu-ehds required_purpose");
        assert!(err.to_string().contains("marketing"));
        // Rejected purpose must not appear as a consent OR an audit event.
        assert!(!deployment.is_consented("patient/42", "marketing"));
        assert!(deployment.audit_events().unwrap().is_empty());
    }
}
