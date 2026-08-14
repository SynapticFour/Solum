//! Consent and access-rights engine for Solum.
//!
//! `solum_profiles::ConsentPolicy` declares **what** a jurisdiction profile
//! requires (workflow variant, required purposes). This crate implements
//! the **runtime state machine**: recording grants and revocations per
//! `(subject, purpose)`, answering "is this access currently consented?",
//! and persisting full history — not just current state — so the EEHRxF
//! individual rights referenced in `docs/roadmap.md` (access, who accessed,
//! onward sharing, rectification) are answerable, not merely enforced.
//!
//! Consent changes should also be emitted as `solum_audit::AuditEvent`s by
//! the caller (e.g. `solum-core`); this crate owns consent *state*, not
//! audit *storage* — see `docs/architecture.md` for the crate boundary.
//!
//! No clinical interpretation happens here — this is purpose/subject
//! bookkeeping only (MDCG boundary, see `CONTRIBUTING.md`).

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solum_profiles::{ConsentWorkflow, JurisdictionProfile};
use thiserror::Error;
use uuid::Uuid;

/// One purpose-scoped consent decision by a data subject.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub id: Uuid,
    pub subject_id: String,
    pub purpose: String,
    /// Data categories this decision covers (typically a subset of a
    /// jurisdiction profile's `encryption.required_field_categories`).
    #[serde(default)]
    pub scope: Vec<String>,
    pub status: ConsentStatus,
    pub recorded_at: DateTime<Utc>,
    /// Who recorded this decision: the subject themself, or an authorised
    /// actor (e.g. emergency-access override). Callers are responsible for
    /// also emitting a `consent.granted` / `consent.revoked` audit event —
    /// this crate does not write to `solum-audit` itself, to keep the two
    /// concerns independently testable.
    pub actor: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentStatus {
    Granted,
    Revoked,
}

#[derive(Debug, Error)]
pub enum ConsentError {
    #[error(
        "purpose '{purpose}' is not among the jurisdiction profile's required_purposes/optional_purposes {allowed:?}"
    )]
    PurposeNotRecognised {
        purpose: String,
        allowed: Vec<String>,
    },
    #[error("consent store I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("consent store serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("consent store corrupt at {path}:{line}: {reason}")]
    Corrupt {
        path: String,
        line: usize,
        reason: String,
    },
}

/// Append-only, file-backed consent history with an in-memory current-state
/// index (last decision per `(subject_id, purpose)`) rebuilt on open.
///
/// Not concurrency-safe across processes (single-writer assumption for
/// stage 1, matching `solum_audit::FileAuditStore`).
#[derive(Debug)]
pub struct ConsentStore {
    path: PathBuf,
    /// subject_id → purpose → last decision. Nested maps avoid allocating a
    /// `(String, String)` key on every lookup.
    current: HashMap<String, HashMap<String, ConsentRecord>>,
    history: Vec<ConsentRecord>,
}

impl ConsentStore {
    /// Open (or create) a consent history file at `path`, replaying all
    /// prior decisions to rebuild current state.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ConsentError> {
        let path = path.as_ref().to_path_buf();
        let mut current: HashMap<String, HashMap<String, ConsentRecord>> = HashMap::new();
        let mut history = Vec::new();
        if path.exists() {
            for record in Self::read_records(&path)? {
                current
                    .entry(record.subject_id.clone())
                    .or_default()
                    .insert(record.purpose.clone(), record.clone());
                history.push(record);
            }
        }
        Ok(Self {
            path,
            current,
            history,
        })
    }

    fn read_records(path: &Path) -> Result<Vec<ConsentRecord>, ConsentError> {
        let file = File::open(path).map_err(|source| ConsentError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for (idx, line) in reader.lines().enumerate() {
            let line = line.map_err(|source| ConsentError::Io {
                path: path.display().to_string(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ConsentRecord =
                serde_json::from_str(&line).map_err(|source| ConsentError::Corrupt {
                    path: path.display().to_string(),
                    line: idx + 1,
                    reason: source.to_string(),
                })?;
            records.push(record);
        }
        Ok(records)
    }

    fn append(&mut self, record: ConsentRecord) -> Result<(), ConsentError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| ConsentError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|source| ConsentError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
        self.current
            .entry(record.subject_id.clone())
            .or_default()
            .insert(record.purpose.clone(), record.clone());
        self.history.push(record);
        Ok(())
    }

    /// Record a grant. Returns the new record.
    pub fn grant(
        &mut self,
        subject_id: impl Into<String>,
        purpose: impl Into<String>,
        scope: Vec<String>,
        actor: impl Into<String>,
    ) -> Result<ConsentRecord, ConsentError> {
        let record = ConsentRecord {
            id: Uuid::new_v4(),
            subject_id: subject_id.into(),
            purpose: purpose.into(),
            scope,
            status: ConsentStatus::Granted,
            recorded_at: Utc::now(),
            actor: actor.into(),
        };
        self.append(record.clone())?;
        Ok(record)
    }

    /// Record a revocation (EEHRxF individual right). Idempotent by design:
    /// revoking an already-revoked or never-granted purpose still writes a
    /// history entry, because Annex II expects a complete trail of the
    /// *attempt*, not just the resulting state.
    pub fn revoke(
        &mut self,
        subject_id: impl Into<String>,
        purpose: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<ConsentRecord, ConsentError> {
        let subject_id = subject_id.into();
        let purpose = purpose.into();
        let scope = self
            .current
            .get(subject_id.as_str())
            .and_then(|m| m.get(purpose.as_str()))
            .map(|r| r.scope.clone())
            .unwrap_or_default();
        let record = ConsentRecord {
            id: Uuid::new_v4(),
            subject_id,
            purpose,
            scope,
            status: ConsentStatus::Revoked,
            recorded_at: Utc::now(),
            actor: actor.into(),
        };
        self.append(record.clone())?;
        Ok(record)
    }

    /// Current status, if any decision has been recorded for this
    /// `(subject_id, purpose)` pair.
    pub fn status(&self, subject_id: &str, purpose: &str) -> Option<ConsentStatus> {
        self.current
            .get(subject_id)
            .and_then(|m| m.get(purpose))
            .map(|r| r.status)
    }

    /// Whether access for `purpose` is currently consented for `subject_id`.
    /// Absence of any record is treated as **not granted** — fail closed.
    pub fn is_granted(&self, subject_id: &str, purpose: &str) -> bool {
        matches!(
            self.status(subject_id, purpose),
            Some(ConsentStatus::Granted)
        )
    }

    /// Current decision record for `(subject_id, purpose)`, if any.
    pub fn current_record(&self, subject_id: &str, purpose: &str) -> Option<&ConsentRecord> {
        self.current.get(subject_id).and_then(|m| m.get(purpose))
    }

    /// Active grant covers `category` when status is Granted and either the
    /// grant's `scope` is empty (purpose-level consent) or contains `category`.
    pub fn is_granted_for_category(&self, subject_id: &str, purpose: &str, category: &str) -> bool {
        match self.current_record(subject_id, purpose) {
            Some(r) if r.status == ConsentStatus::Granted => {
                r.scope.is_empty() || r.scope.iter().any(|s| s == category)
            }
            _ => false,
        }
    }

    /// Full decision history for one subject, in recorded order — backs the
    /// EEHRxF "access, who accessed, onward sharing" transparency right.
    pub fn history_for_subject(
        &self,
        subject_id: &str,
    ) -> Result<Vec<ConsentRecord>, ConsentError> {
        Ok(self
            .history
            .iter()
            .filter(|r| r.subject_id == subject_id)
            .cloned()
            .collect())
    }
}

/// Validate that `purpose` is one the active jurisdiction profile
/// recognises (`required_purposes` ∪ `optional_purposes`). Call before
/// [`ConsentStore::grant`] so an unrecognised purpose fails before it is
/// persisted, not after.
pub fn validate_purpose(profile: &JurisdictionProfile, purpose: &str) -> Result<(), ConsentError> {
    let recognised = profile
        .consent
        .required_purposes
        .iter()
        .chain(profile.consent.optional_purposes.iter())
        .any(|p| p == purpose);
    if recognised {
        Ok(())
    } else {
        let allowed: Vec<String> = profile
            .consent
            .required_purposes
            .iter()
            .chain(profile.consent.optional_purposes.iter())
            .cloned()
            .collect();
        Err(ConsentError::PurposeNotRecognised {
            purpose: purpose.to_string(),
            allowed,
        })
    }
}

/// Whether this crate's record-keeping model can back a given
/// [`ConsentWorkflow`] variant. Grant/revoke + purpose binding is the
/// shared state machine for all four variants; workflow-specific UX
/// (witness capture, dynamic re-consent prompts) remains a caller concern.
pub fn supports_workflow(_workflow: &ConsentWorkflow) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, ConsentStore) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("consent.jsonl");
        let store = ConsentStore::open(&path).unwrap();
        (dir, store)
    }

    #[test]
    fn grant_then_query() {
        let (_dir, mut store) = temp_store();
        store
            .grant(
                "patient/42",
                "care_provision",
                vec!["patient_summary".into()],
                "practitioner/7",
            )
            .unwrap();
        assert!(store.is_granted("patient/42", "care_provision"));
        assert!(!store.is_granted("patient/42", "secondary_use_hdab"));
    }

    #[test]
    fn revoke_flips_status() {
        let (_dir, mut store) = temp_store();
        store
            .grant("patient/42", "care_provision", vec![], "patient/42")
            .unwrap();
        assert!(store.is_granted("patient/42", "care_provision"));

        store
            .revoke("patient/42", "care_provision", "patient/42")
            .unwrap();
        assert!(!store.is_granted("patient/42", "care_provision"));
        assert_eq!(
            store.status("patient/42", "care_provision"),
            Some(ConsentStatus::Revoked)
        );
    }

    #[test]
    fn unknown_subject_purpose_is_not_granted() {
        let (_dir, store) = temp_store();
        assert!(!store.is_granted("patient/999", "care_provision"));
    }

    #[test]
    fn revoking_ungranted_purpose_still_records_history() {
        let (_dir, mut store) = temp_store();
        store
            .revoke("patient/42", "care_provision", "practitioner/7")
            .unwrap();
        let history = store.history_for_subject("patient/42").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, ConsentStatus::Revoked);
    }

    #[test]
    fn state_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("consent.jsonl");
        {
            let mut store = ConsentStore::open(&path).unwrap();
            store
                .grant("patient/42", "care_provision", vec![], "patient/42")
                .unwrap();
        }
        let store = ConsentStore::open(&path).unwrap();
        assert!(store.is_granted("patient/42", "care_provision"));
    }

    #[test]
    fn history_orders_multiple_decisions() {
        let (_dir, mut store) = temp_store();
        store
            .grant("patient/42", "care_provision", vec![], "patient/42")
            .unwrap();
        store
            .revoke("patient/42", "care_provision", "patient/42")
            .unwrap();
        store
            .grant("patient/42", "care_provision", vec![], "patient/42")
            .unwrap();

        let history = store.history_for_subject("patient/42").unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].status, ConsentStatus::Granted);
        assert_eq!(history[1].status, ConsentStatus::Revoked);
        assert_eq!(history[2].status, ConsentStatus::Granted);
        assert!(store.is_granted("patient/42", "care_provision"));
    }

    #[test]
    fn validate_purpose_against_profile() {
        let toml = r#"
schema_version = 1
[meta]
profile = "test"
jurisdiction = "EU"
description = "test"
[encryption]
required_field_categories = ["patient_summary"]
allowed_key_custody = ["customer_held"]
[audit]
mandatory_events = ["access.granted"]
[retention]
default_retention_days = 3650
audit_log_retention_days = 3650
[storage]
allowed_regions = ["EU"]
enforce_residency = true
[consent]
workflow = "gdpr_granular"
required_purposes = ["care_provision"]
"#;
        let profile = solum_profiles::parse_profile_str(toml, "test").unwrap();
        assert!(validate_purpose(&profile, "care_provision").is_ok());
        let err = validate_purpose(&profile, "marketing").expect_err("must reject");
        assert!(matches!(err, ConsentError::PurposeNotRecognised { .. }));
    }
}
