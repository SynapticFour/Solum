//! Hospital IdP profiles (Keycloak, Entra, SMART Backend Services).
//!
//! Solum does **not** issue GA4GH Passports. These profiles map clinician or
//! system tokens onto [`OrgCapMapping`] so consent/audit bind to the IdP `sub`.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Built-in profile names (files under `config/idp-profiles/`).
pub const KNOWN_IDP_PROFILES: &[&str] = &["keycloak-hospital", "entra", "smart-backend"];

/// Operator IdP profile (claims-map + org-IAM file).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct IdpProfile {
    pub name: String,
    #[serde(default)]
    pub issuer_contains: String,
    #[serde(default = "default_groups_claim")]
    pub groups_claim: String,
    #[serde(default)]
    pub audience: String,
    /// `oidc` (clinician login) or `smart-backend` (system client credentials).
    #[serde(default = "default_flow")]
    pub flow: String,
    #[serde(default)]
    pub org_iam: String,
}

fn default_groups_claim() -> String {
    "groups".into()
}

fn default_flow() -> String {
    "oidc".into()
}

impl IdpProfile {
    /// Load from a TOML file.
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read IdP profile {}: {e}", path.display()))?;
        toml::from_str(&raw).map_err(|e| format!("invalid IdP profile {}: {e}", path.display()))
    }

    /// `config/idp-profiles/<name>.toml` relative to `config_root`.
    pub fn load_named(config_root: &Path, name: &str) -> Result<Self, String> {
        let key = name.trim().to_lowercase();
        if !KNOWN_IDP_PROFILES.contains(&key.as_str()) {
            return Err(format!(
                "unknown IdP profile '{name}' (expected {})",
                KNOWN_IDP_PROFILES.join(", ")
            ));
        }
        let path = config_root.join("idp-profiles").join(format!("{key}.toml"));
        Self::load_from_path(&path)
    }

    /// Org-IAM mapping path (relative to the process working directory).
    pub fn org_iam_path(&self) -> PathBuf {
        PathBuf::from(&self.org_iam)
    }
}

/// Resolve `auto` from the issuer URL; otherwise return the named profile if known.
pub fn detect_idp_profile(issuer: &str, configured: &str) -> Option<String> {
    let explicit = configured.trim().to_lowercase();
    if !explicit.is_empty() && explicit != "auto" {
        if KNOWN_IDP_PROFILES.contains(&explicit.as_str()) {
            return Some(explicit);
        }
        return None;
    }
    let iss = issuer.to_lowercase();
    if iss.contains("login.microsoftonline.com") || iss.contains("sts.windows.net") {
        return Some("entra".into());
    }
    if iss.contains("/realms/") {
        return Some("keycloak-hospital".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config")
    }

    #[test]
    fn loads_entra_and_keycloak() {
        let entra = IdpProfile::load_named(&config_root(), "entra").unwrap();
        assert_eq!(entra.groups_claim, "groups");
        assert_eq!(entra.flow, "oidc");
        let kc = IdpProfile::load_named(&config_root(), "keycloak-hospital").unwrap();
        assert_eq!(kc.groups_claim, "realm_access.roles");
        let smart = IdpProfile::load_named(&config_root(), "smart-backend").unwrap();
        assert_eq!(smart.flow, "smart-backend");
        assert_eq!(kc.org_iam, "config/org-iam/keycloak-hospital.toml");
        let mapping = crate::OrgCapMapping::load_from_path(
            &config_root().join("org-iam/keycloak-hospital.toml"),
        )
        .unwrap();
        assert_eq!(mapping.claim_path, "realm_access.roles");
    }

    #[test]
    fn detect_from_issuer() {
        assert_eq!(
            detect_idp_profile("https://login.microsoftonline.com/tenant/v2.0", "auto"),
            Some("entra".into())
        );
        assert_eq!(
            detect_idp_profile("https://idp.klinik.de/realms/hospital", "auto"),
            Some("keycloak-hospital".into())
        );
        assert_eq!(
            detect_idp_profile("https://example.com", "entra"),
            Some("entra".into())
        );
    }

    #[test]
    fn rejects_unknown_name() {
        assert!(IdpProfile::load_named(&config_root(), "google").is_err());
    }
}
