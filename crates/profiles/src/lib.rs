//! Jurisdiction profile loader and startup conformance checks.
//!
//! Profiles are TOML files under `config/profiles/`. Adding a new jurisdiction
//! (e.g. Kenya, Nigeria, South Africa) is a **data** change: drop a new TOML
//! file matching [`JurisdictionProfile`]. No code change is required unless
//! the schema itself is extended.
//!
//! On startup, [`validate_startup`] compares the declared profile against the
//! actual deployment configuration. Mismatches **refuse to start** — they are
//! not merely logged.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use solum_crypto::{validate_key_custody, KeyCustody, KeyManagementConfig};
use thiserror::Error;

/// Schema version for jurisdiction profile TOML files.
pub const PROFILE_SCHEMA_VERSION: u32 = 1;

/// A jurisdiction profile loaded from TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JurisdictionProfile {
    pub schema_version: u32,
    pub meta: ProfileMeta,
    pub encryption: EncryptionPolicy,
    pub audit: AuditPolicy,
    pub retention: RetentionPolicy,
    pub storage: StoragePolicy,
    pub consent: ConsentPolicy,
    /// Optional Annex / regulation references for documentation and audits.
    #[serde(default)]
    pub regulatory: RegulatoryRefs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileMeta {
    /// Stable profile id (filename stem convention: `eu-ehds`, `ke-dpa`, …).
    pub profile: String,
    pub jurisdiction: String,
    pub description: String,
    /// Legal / standards references (human-readable).
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptionPolicy {
    /// Clinical / administrative field categories that must be encrypted at rest.
    pub required_field_categories: Vec<String>,
    /// Allowed key-custody modes for this jurisdiction.
    pub allowed_key_custody: Vec<KeyCustody>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditPolicy {
    /// Audit event types that must be emitted (refuse start if not configured).
    pub mandatory_events: Vec<String>,
    /// Whether an external evidence tool (e.g. HELIOS) may be attached.
    #[serde(default)]
    pub helios_export_prepared: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Default retention period in days for clinical records.
    pub default_retention_days: u32,
    /// Minimum retention for audit / access logs in days.
    pub audit_log_retention_days: u32,
    /// Optional per-category overrides: category → days.
    #[serde(default)]
    pub category_overrides_days: std::collections::BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoragePolicy {
    /// ISO-like region codes or labels allowed for primary data residency
    /// (e.g. `EU`, `EEA`, `DE`, `KE`).
    pub allowed_regions: Vec<String>,
    /// If true, cross-border storage outside `allowed_regions` is forbidden.
    pub enforce_residency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentWorkflow {
    /// GDPR / EHDS-style granular opt-in with purpose limitation.
    GdprGranular,
    /// Explicit written / recorded consent with witness option.
    ExplicitRecorded,
    /// Dynamic consent with revocable purposes.
    DynamicRevocable,
    /// Secondary-use pathway via a health data access body (HDAB-style).
    HdabSecondaryUse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentPolicy {
    pub workflow: ConsentWorkflow,
    /// Purposes that must be expressible in the consent record.
    #[serde(default)]
    pub required_purposes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegulatoryRefs {
    /// e.g. EHDS Annex II section identifiers.
    #[serde(default)]
    pub annex_requirements: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Actual deployment configuration checked against a profile at startup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Primary storage region / residency label of this deployment.
    pub storage_region: String,
    pub key_management: KeyManagementConfig,
    /// Audit event types the deployment has enabled.
    pub enabled_audit_events: Vec<String>,
    /// Consent workflow implemented by the deployment.
    pub consent_workflow: ConsentWorkflow,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("failed to read profile at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse profile TOML at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("unsupported profile schema_version {found} (expected {PROFILE_SCHEMA_VERSION})")]
    UnsupportedSchema { found: u32 },
    #[error(
        "startup refused: configuration contradicts jurisdiction profile '{profile}': {reason}"
    )]
    StartupRefused { profile: String, reason: String },
}

/// Load a jurisdiction profile from a TOML file.
pub fn load_profile(path: impl AsRef<Path>) -> Result<JurisdictionProfile, ProfileError> {
    let path_ref = path.as_ref();
    let path_str = path_ref.display().to_string();
    let raw = fs::read_to_string(path_ref).map_err(|source| ProfileError::Io {
        path: path_str.clone(),
        source,
    })?;
    parse_profile_str(&raw, &path_str)
}

/// Parse a profile from a TOML string (tests / embedded fixtures).
pub fn parse_profile_str(raw: &str, path_hint: &str) -> Result<JurisdictionProfile, ProfileError> {
    let profile: JurisdictionProfile =
        toml::from_str(raw).map_err(|source| ProfileError::Parse {
            path: path_hint.to_string(),
            source,
        })?;
    if profile.schema_version != PROFILE_SCHEMA_VERSION {
        return Err(ProfileError::UnsupportedSchema {
            found: profile.schema_version,
        });
    }
    Ok(profile)
}

/// Load every `*.toml` profile from a directory (extensible without code changes).
pub fn load_profiles_dir(dir: impl AsRef<Path>) -> Result<Vec<JurisdictionProfile>, ProfileError> {
    let dir = dir.as_ref();
    let mut profiles = Vec::new();
    let entries = fs::read_dir(dir).map_err(|source| ProfileError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ProfileError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            profiles.push(load_profile(&path)?);
        }
    }
    profiles.sort_by(|a, b| a.meta.profile.cmp(&b.meta.profile));
    Ok(profiles)
}

/// Validate runtime configuration against a jurisdiction profile.
///
/// Returns `Ok(())` only if storage residency, key custody, mandatory audit
/// events, and consent workflow all match. Any contradiction yields
/// [`ProfileError::StartupRefused`] — callers must abort process startup.
pub fn validate_startup(
    profile: &JurisdictionProfile,
    runtime: &RuntimeConfig,
) -> Result<(), ProfileError> {
    let refuse = |reason: String| ProfileError::StartupRefused {
        profile: profile.meta.profile.clone(),
        reason,
    };

    if profile.storage.enforce_residency {
        let region = runtime.storage_region.to_uppercase();
        let allowed: Vec<String> = profile
            .storage
            .allowed_regions
            .iter()
            .map(|r| r.to_uppercase())
            .collect();
        if !allowed.iter().any(|r| r == &region) {
            return Err(refuse(format!(
                "storage_region '{region}' is not in allowed_regions {allowed:?}"
            )));
        }
    }

    validate_key_custody(
        &runtime.key_management,
        &profile.encryption.allowed_key_custody,
    )
    .map_err(|e| refuse(e.to_string()))?;

    for required in &profile.audit.mandatory_events {
        if !runtime
            .enabled_audit_events
            .iter()
            .any(|e| e.eq_ignore_ascii_case(required))
        {
            return Err(refuse(format!(
                "mandatory audit event '{required}' is not enabled"
            )));
        }
    }

    if runtime.consent_workflow != profile.consent.workflow {
        return Err(refuse(format!(
            "consent workflow {:?} does not match profile requirement {:?}",
            runtime.consent_workflow, profile.consent.workflow
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solum_crypto::KeyManagementConfig;
    use std::path::PathBuf;

    fn profiles_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles")
    }

    fn eu_ehds() -> JurisdictionProfile {
        load_profile(profiles_dir().join("eu-ehds.toml")).expect("eu-ehds.toml must load")
    }

    fn conforming_runtime(profile: &JurisdictionProfile) -> RuntimeConfig {
        RuntimeConfig {
            storage_region: profile.storage.allowed_regions[0].clone(),
            key_management: KeyManagementConfig {
                custody: KeyCustody::CustomerHeld,
                provider: Some("customer-hsm-eu".into()),
            },
            enabled_audit_events: profile.audit.mandatory_events.clone(),
            consent_workflow: profile.consent.workflow.clone(),
        }
    }

    #[test]
    fn loads_eu_ehds_profile() {
        let p = eu_ehds();
        assert_eq!(p.meta.profile, "eu-ehds");
        assert_eq!(p.schema_version, PROFILE_SCHEMA_VERSION);
        assert!(p.storage.allowed_regions.iter().any(|r| r == "EU"));
        assert!(!p.encryption.required_field_categories.is_empty());
        assert!(!p.audit.mandatory_events.is_empty());
        assert!(!p.regulatory.annex_requirements.is_empty());
    }

    #[test]
    fn conforming_config_starts() {
        let p = eu_ehds();
        let runtime = conforming_runtime(&p);
        assert!(validate_startup(&p, &runtime).is_ok());
    }

    #[test]
    fn refuses_non_eu_storage_region() {
        let p = eu_ehds();
        let mut runtime = conforming_runtime(&p);
        runtime.storage_region = "us-east-1".into();
        let err = validate_startup(&p, &runtime).expect_err("must refuse non-EU storage");
        match err {
            ProfileError::StartupRefused { profile, reason } => {
                assert_eq!(profile, "eu-ehds");
                assert!(
                    reason.contains("storage_region"),
                    "reason should mention storage_region: {reason}"
                );
                assert!(
                    reason.contains("US-EAST-1") || reason.contains("us-east-1"),
                    "reason should mention the bad region: {reason}"
                );
            }
            other => panic!("expected StartupRefused, got {other:?}"),
        }
    }

    #[test]
    fn refuses_wrong_key_custody() {
        let p = eu_ehds();
        let mut runtime = conforming_runtime(&p);
        runtime.key_management.custody = KeyCustody::OperatorHeld;
        let err = validate_startup(&p, &runtime).expect_err("must refuse operator-held keys");
        assert!(matches!(err, ProfileError::StartupRefused { .. }));
    }

    #[test]
    fn refuses_missing_mandatory_audit_event() {
        let p = eu_ehds();
        let mut runtime = conforming_runtime(&p);
        runtime.enabled_audit_events.clear();
        let err = validate_startup(&p, &runtime).expect_err("must refuse missing audit events");
        assert!(matches!(err, ProfileError::StartupRefused { .. }));
    }
}
