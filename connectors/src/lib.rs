//! Track A connector seam beside an existing KIS.
//!
//! The sidecar remains the HTTP process (`grant` / `revoke` / status, field
//! crypto, audit). This crate does not open a socket, does not grant consent,
//! and does not name a hospital system. A blank or unknown value is an error.

#![forbid(unsafe_code)]

mod mock;

pub use mock::MockKisConnector;

/// What the KIS export says about one subject and purpose.
///
/// `granted: None` means the export did not state a decision. That is not a denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KisConsentClaim {
    pub subject_id: String,
    pub purpose: String,
    pub granted: Option<bool>,
}

/// What the Track A sidecar already recorded for the same pair.
///
/// `granted: None` means there is no sidecar record. That is not a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarConsent {
    pub subject_id: String,
    pub purpose: String,
    pub granted: Option<bool>,
}

/// The only successful consent outcome: both sides state the same decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentAlignment {
    pub granted: bool,
}

/// One populated field from a KIS export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceField {
    pub name: String,
    pub value: String,
}

/// A source field whose name is in this connector's table.
///
/// `solum_field` is a Solum-side name for the sidecar, not a claim that the
/// value already matches a FHIR profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedField {
    pub source_name: String,
    pub solum_field: String,
    pub value: String,
}

/// Fail-closed outcome. No variant means "retry against the live KIS".
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectorError {
    #[error("invalid connector input: {0}")]
    InvalidInput(&'static str),
    #[error("consent was not stated by {side}")]
    ConsentUnstated { side: &'static str },
    #[error("consent mismatch: KIS says {kis_granted}, sidecar says {sidecar_granted}")]
    ConsentMismatch {
        kis_granted: bool,
        sidecar_granted: bool,
    },
    #[error("no mapping for source field {name}")]
    UnmappedField { name: String },
    #[error("source field {name} is empty")]
    EmptyValue { name: String },
}

/// What the caller does with [`ConnectorError`]. The template always stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDisposition {
    pub reason: &'static str,
}

/// Generic KIS connector for a Track A sidecar integration.
///
/// Implementors translate one system's export. They do not replace the KIS
/// and they do not speak for a named product.
pub trait KisConnector {
    /// Compare a KIS claim with a sidecar record the caller already loaded.
    ///
    /// Ok only when both sides state the same grant or the same refusal.
    /// A missing statement or a disagreement is [`ConnectorError`].
    fn reconcile_consent(
        &self,
        kis: &KisConsentClaim,
        sidecar: &SidecarConsent,
    ) -> Result<ConsentAlignment, ConnectorError>;

    /// Map populated source fields through this connector's exact-name table.
    ///
    /// An unknown name or an empty value fails the whole call. Partial maps
    /// are not returned.
    fn map_fields(&self, fields: &[SourceField]) -> Result<Vec<MappedField>, ConnectorError>;

    /// Classify an error for the caller. This template does not retry.
    fn classify_error(&self, error: &ConnectorError) -> ErrorDisposition;
}
