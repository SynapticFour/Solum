//! H3.2 migration helpers — FHIR batch import into Track B façade stores / CDR.

use std::fs;
use std::path::Path;

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Extract allowlisted FHIR resources from a Bundle or single resource JSON file.
pub fn extract_fhir_resources(doc: &Value) -> Result<Vec<Value>, MigrateError> {
    const ALLOWED: &[&str] = &[
        "Bundle",
        "Composition",
        "Patient",
        "AllergyIntolerance",
        "MedicationStatement",
        "Condition",
    ];
    let rtype = doc
        .get("resourceType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MigrateError::Message("missing resourceType".into()))?;
    if rtype == "Bundle" {
        let entries = doc
            .get("entry")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for e in entries {
            if let Some(res) = e.get("resource") {
                let rt = res
                    .get("resourceType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if ALLOWED.contains(&rt) && rt != "Bundle" {
                    out.push(res.clone());
                }
            }
        }
        return Ok(out);
    }
    if ALLOWED.contains(&rtype) {
        return Ok(vec![doc.clone()]);
    }
    Err(MigrateError::Message(format!(
        "resourceType '{rtype}' not in H3.1/H3.2 allowlist"
    )))
}

/// Idempotency key: resourceType/id (generates placeholder id when missing).
pub fn resource_idempotency_key(resource: &Value) -> String {
    let rt = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let id = resource
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("missing-id");
    format!("{rt}/{id}")
}

/// Append a dual-write dead-letter record (never silent drop).
pub fn append_dead_letter(path: impl AsRef<Path>, record: &Value) -> Result<(), MigrateError> {
    use std::io::Write;
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(record)?)?;
    Ok(())
}

/// Build a standard dead-letter JSONL row (reason + timestamp + payload).
pub fn dead_letter_row(reason: &str, payload: &Value) -> Value {
    serde_json::json!({
        "reason": reason,
        "at": chrono::Utc::now().to_rfc3339(),
        "payload": payload,
    })
}

/// Load FHIR JSON from path.
pub fn load_fhir_json(path: impl AsRef<Path>) -> Result<Value, MigrateError> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_bundle_entries() {
        let doc = json!({
            "resourceType": "Bundle",
            "entry": [
                {"resource": {"resourceType": "Patient", "id": "p1"}},
                {"resource": {"resourceType": "Observation", "id": "o1"}},
            ]
        });
        let got = extract_fhir_resources(&doc).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(resource_idempotency_key(&got[0]), "Patient/p1");
    }
}
