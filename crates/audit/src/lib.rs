//! Audit event recording and evidence export hooks for Solum.
//!
//! HELIOS (`helios-audit`) is a separate Apache-2.0 evidence tool in the
//! Synaptic Four portfolio. This crate prepares a stable JSON export shape
//! so HELIOS (or an equivalent) can consume Solum audit trails without
//! embedding HELIOS as a Rust dependency.

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Denied,
    Error,
}

/// In-memory audit buffer (replace with durable store in later stages).
#[derive(Debug, Default)]
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
