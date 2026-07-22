//! Solum crypto layer: customer-held keys and field-level encryption policy.
//!
//! Reuses Ferrum sovereignty building blocks via a git-pinned `ferrum-core`
//! dependency (Lab Kit pattern). Crypt4GH-style envelope encryption and
//! clinical field policies that Ferrum does not provide belong **here**,
//! not in Ferrum upstream.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Git revision pinned in `Cargo.toml` (mirror `config/ci/ferrum-revision.txt`).
pub const FERRUM_GIT_REV: &str = "27a6a8e9a719fd1a171da28b20462a777f95cf65";

/// Upstream repository URL.
pub const FERRUM_GIT_URL: &str = "https://github.com/SynapticFour/Ferrum.git";

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

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key custody {actual:?} is not allowed by the active jurisdiction profile (allowed: {allowed:?})")]
    CustodyNotAllowed {
        actual: KeyCustody,
        allowed: Vec<KeyCustody>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
