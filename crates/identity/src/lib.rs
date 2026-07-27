//! Structured actor identity for Solum (Sprint 2 auth adapter).
//!
//! # Why `solum-identity` (not `solum-audit`)
//!
//! `AuditEvent.actor` and `ConsentRecord.actor` remain `String` — that is the
//! already-exported HELIOS / consent-store format and must stay stable across
//! baselines. [`SolumActor`] is an *additional* in-memory representation that
//! maps onto that string via [`SolumActor::to_audit_string`]; it is not a
//! replacement for the storage field type.
//!
//! Placing the type in `solum-audit` would either pull optional AuthClaims
//! mapping into the evidence-export crate or force `solum-consent` to depend
//! on audit just to share an auth-adapter type — breaking the documented
//! boundary that consent state and audit storage stay independently testable.
//! A slim identity crate owns the adapter; audit and consent keep writing
//! plain `actor: String` values produced by callers (typically `Deployment`).
//!
//! # Feature `ferrum-companion`
//!
//! Gates only whether [`TryFrom`]`<&ferrum_core::auth::AuthClaims>` is
//! compiled. It does **not** decide whether `ferrum-core` is linked into the
//! Solum workspace — that hard dependency already lives in `solum-crypto`
//! (shared git pin). Enabling this feature merely compiles the AuthClaims →
//! [`SolumActor`] mapping on top of that existing link path.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Where a [`SolumActor`] was constructed from (auth world), not a storage tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorSource {
    FerrumPassport,
    Standalone,
    LocalDev,
}

/// Rich in-memory actor identity. Persist via [`SolumActor::to_audit_string`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolumActor {
    pub subject_id: String,
    pub display: Option<String>,
    pub source: ActorSource,
    pub scopes: Vec<String>,
}

impl SolumActor {
    /// Mode A / SMART-on-FHIR-shaped construction — no live token verification
    /// (Sprint 5). Analogous to the Sprint-1 AuthClaims construction smoke.
    pub fn standalone(subject_id: impl Into<String>, scopes: Vec<String>) -> Self {
        Self {
            subject_id: subject_id.into(),
            display: None,
            source: ActorSource::Standalone,
            scopes,
        }
    }

    /// Canonical `actor` string for `AuditEvent` / `ConsentRecord` storage.
    ///
    /// `LocalDev` (including [`From<String>`] / [`From<&str>`]) returns the
    /// subject id unchanged so fixtures like `"practitioner/7"` export
    /// identically to pre-Sprint-2 baselines.
    pub fn to_audit_string(&self) -> String {
        match self.source {
            ActorSource::FerrumPassport => format!("ferrum:passport:{}", self.subject_id),
            ActorSource::Standalone => format!("standalone:{}", self.subject_id),
            ActorSource::LocalDev => self.subject_id.clone(),
        }
    }
}

impl From<String> for SolumActor {
    fn from(subject_id: String) -> Self {
        Self {
            subject_id,
            display: None,
            source: ActorSource::LocalDev,
            scopes: Vec::new(),
        }
    }
}

impl From<&str> for SolumActor {
    fn from(subject_id: &str) -> Self {
        Self::from(subject_id.to_string())
    }
}

/// Errors when mapping Ferrum [`AuthClaims`](ferrum_core::auth::AuthClaims) into
/// a [`SolumActor`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActorFromClaimsError {
    #[error("AuthClaims missing subject (sub)")]
    MissingSubject,
}

#[cfg(feature = "ferrum-companion")]
impl TryFrom<&solum_crypto::ferrum_core::auth::AuthClaims> for SolumActor {
    type Error = ActorFromClaimsError;

    fn try_from(claims: &solum_crypto::ferrum_core::auth::AuthClaims) -> Result<Self, Self::Error> {
        use solum_crypto::ferrum_core::auth::AuthClaims;

        let subject_id = claims
            .sub()
            .ok_or(ActorFromClaimsError::MissingSubject)?
            .to_string();

        let scope_raw = match claims {
            AuthClaims::Jwt { scope, .. } => scope.as_deref(),
            AuthClaims::Passport { claims, .. } => claims.scope.as_deref(),
        };
        let scopes = scope_raw
            .map(|s| {
                s.split_whitespace()
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            subject_id,
            display: None,
            source: ActorSource::FerrumPassport,
            scopes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_to_audit_string_is_identity() {
        let original = "practitioner/7".to_string();
        let actor = SolumActor::from(original.clone());
        assert_eq!(actor.source, ActorSource::LocalDev);
        assert!(actor.scopes.is_empty());
        assert_eq!(actor.display, None);
        assert_eq!(actor.to_audit_string(), original);
    }

    #[test]
    fn from_str_to_audit_string_is_identity() {
        let actor = SolumActor::from("practitioner/7");
        assert_eq!(actor.to_audit_string(), "practitioner/7");
    }

    #[test]
    fn standalone_sets_source_scopes_and_audit_prefix() {
        let actor = SolumActor::standalone(
            "practitioner/7",
            vec!["patient/*.read".into(), "launch/patient".into()],
        );
        assert_eq!(actor.source, ActorSource::Standalone);
        assert_eq!(actor.subject_id, "practitioner/7");
        assert_eq!(
            actor.scopes,
            vec!["patient/*.read".to_string(), "launch/patient".to_string()]
        );
        assert_eq!(actor.to_audit_string(), "standalone:practitioner/7");
    }

    #[test]
    fn ferrum_passport_audit_string_prefix() {
        let actor = SolumActor {
            subject_id: "researcher@example.org".into(),
            display: None,
            source: ActorSource::FerrumPassport,
            scopes: vec!["drs.read".into()],
        };
        assert_eq!(
            actor.to_audit_string(),
            "ferrum:passport:researcher@example.org"
        );
    }
}
