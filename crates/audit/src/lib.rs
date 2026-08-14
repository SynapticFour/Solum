//! Audit event recording and evidence export hooks for Solum.
//!
//! HELIOS (`helios-audit`) is a separate Apache-2.0 evidence tool in the
//! Synaptic Four portfolio. This crate prepares a stable JSON export shape
//! so HELIOS (or an equivalent) can consume Solum audit trails without
//! embedding HELIOS as a Rust dependency.
//!
//! Two log types are available: [`AuditLog`] is an in-memory buffer (tests,
//! short-lived processes); [`FileAuditStore`] is the durable, hash-chained,
//! tamper-evident log intended for real deployments — see its own docs and
//! `docs/architecture.md`.

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod store;
pub use store::{AuditRecord, AuditStoreError, FileAuditStore, GENESIS_HASH};

/// Stable event-type strings required by jurisdiction profiles and written by
/// [`crate::FileAuditStore`] callers (`Deployment`, sidecar).
pub mod events {
    pub const ACCESS_GRANTED: &str = "access.granted";
    pub const ACCESS_DENIED: &str = "access.denied";
    pub const DATA_READ: &str = "data.read";
    pub const DATA_EXPORT: &str = "data.export";
    pub const DATA_RECEIVE_EEHRXF: &str = "data.receive_eehrxf";
    pub const CONSENT_GRANTED: &str = "consent.granted";
    pub const CONSENT_REVOKED: &str = "consent.revoked";
    pub const CONSENT_DENIED: &str = "consent.denied";
    pub const IDENTITY_AUTHENTICATED: &str = "identity.authenticated";
    pub const KEY_USE: &str = "key.use";
    pub const RESIDENCY_TRANSFER_ATTEMPT: &str = "residency.transfer_attempt";
    pub const DATA_ENCRYPT: &str = "data.encrypt";
    pub const DATA_DECRYPT: &str = "data.decrypt";

    pub const CDR_TEMPLATE_UPLOADED: &str = "cdr.template.uploaded";
    pub const CDR_EHR_CREATED: &str = "cdr.ehr.created";
    pub const CDR_COMPOSITION_COMMITTED: &str = "cdr.composition.committed";
    pub const CDR_AQL_EXECUTED: &str = "cdr.aql.executed";
    pub const CDR_FHIR_CREATED: &str = "cdr.fhir.created";
    pub const CDR_SUBJECT_LINK_UPSERTED: &str = "cdr.subject_link.upserted";
    pub const CDR_DUAL_WRITE_OK: &str = "cdr.dual_write.ok";
    pub const CDR_DUAL_WRITE_DEAD_LETTERED: &str = "cdr.dual_write.dead_lettered";

    /// Event types the product actually writes (startup checklists must be a subset).
    pub const PRODUCT_EMITTED: &[&str] = &[
        ACCESS_GRANTED,
        ACCESS_DENIED,
        DATA_READ,
        DATA_EXPORT,
        DATA_RECEIVE_EEHRXF,
        CONSENT_GRANTED,
        CONSENT_REVOKED,
        CONSENT_DENIED,
        IDENTITY_AUTHENTICATED,
        KEY_USE,
        RESIDENCY_TRANSFER_ATTEMPT,
        DATA_ENCRYPT,
        DATA_DECRYPT,
        CDR_TEMPLATE_UPLOADED,
        CDR_EHR_CREATED,
        CDR_COMPOSITION_COMMITTED,
        CDR_AQL_EXECUTED,
        CDR_FHIR_CREATED,
        CDR_SUBJECT_LINK_UPSERTED,
        CDR_DUAL_WRITE_OK,
        CDR_DUAL_WRITE_DEAD_LETTERED,
    ];
}

/// A single auditable event required by jurisdiction profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    /// Clinical / administrative data category touched (if any).
    pub data_category: Option<String>,
    pub outcome: AuditOutcome,
    #[serde(default)]
    pub details: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Denied,
    Error,
    /// Operation was attempted but failed (e.g. decrypt with wrong key).
    /// Distinct from [`Denied`] (policy refusal) and [`Error`] (infrastructure).
    Failure,
}

/// In-memory audit buffer for unit tests. Product paths use [`FileAuditStore`].
#[derive(Debug, Default)]
#[doc(hidden)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, event: AuditEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Export events as JSON suitable for HELIOS / external evidence pipelines.
    pub fn export_helios_json(&self) -> Result<String, serde_json::Error> {
        let envelope = HeliosExportEnvelope {
            format: "solum-audit-helios-v1",
            generator: "solum-audit",
            events: self.events.clone(),
        };
        serde_json::to_string_pretty(&envelope)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeliosExportEnvelope {
    format: &'static str,
    generator: &'static str,
    events: Vec<AuditEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helios_export_roundtrip_shape() {
        let mut log = AuditLog::new();
        log.record(AuditEvent {
            event_type: "access.granted".into(),
            timestamp: Utc::now(),
            actor: "practitioner/123".into(),
            data_category: Some("patient_summary".into()),
            outcome: AuditOutcome::Success,
            details: Default::default(),
        });
        let json = log.export_helios_json().unwrap();
        assert!(json.contains("solum-audit-helios-v1"));
        assert!(json.contains("access.granted"));
    }
}
