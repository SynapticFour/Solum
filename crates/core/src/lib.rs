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
}
