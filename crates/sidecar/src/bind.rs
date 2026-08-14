//! Object-level authorization: consented subject must own the resource.

use serde_json::Value;

/// Whether a stored FHIR resource belongs to `subject` (fail-closed).
pub fn fhir_resource_belongs_to_subject(
    resource_type: &str,
    id: &str,
    resource: &Value,
    subject: &str,
) -> bool {
    if subject.is_empty() {
        return false;
    }
    if resource_type == "Patient" {
        return id == subject || resource.get("id").and_then(|v| v.as_str()) == Some(subject);
    }
    if let Some(r) = resource
        .pointer("/subject/reference")
        .and_then(|v| v.as_str())
    {
        return reference_matches_subject(r, subject);
    }
    if let Some(r) = resource
        .pointer("/patient/reference")
        .and_then(|v| v.as_str())
    {
        return reference_matches_subject(r, subject);
    }
    false
}

fn reference_matches_subject(reference: &str, subject: &str) -> bool {
    if reference == subject {
        return true;
    }
    let tail = reference.rsplit('/').next().unwrap_or(reference);
    tail == subject || reference == format!("Patient/{subject}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn patient_id_must_match() {
        let r = json!({"resourceType": "Patient", "id": "patient/42"});
        assert!(fhir_resource_belongs_to_subject(
            "Patient",
            "patient/42",
            &r,
            "patient/42"
        ));
        assert!(!fhir_resource_belongs_to_subject(
            "Patient",
            "patient/42",
            &r,
            "patient/99"
        ));
    }

    #[test]
    fn observation_subject_reference() {
        let r = json!({
            "resourceType": "Observation",
            "id": "obs-1",
            "subject": {"reference": "Patient/patient/42"}
        });
        assert!(fhir_resource_belongs_to_subject(
            "Observation",
            "obs-1",
            &r,
            "patient/42"
        ));
        assert!(!fhir_resource_belongs_to_subject(
            "Observation",
            "obs-1",
            &r,
            "patient/99"
        ));
    }

    #[test]
    fn observation_without_subject_denied() {
        let r = json!({"resourceType": "Observation", "id": "obs-1"});
        assert!(!fhir_resource_belongs_to_subject(
            "Observation",
            "obs-1",
            &r,
            "patient/42"
        ));
    }
}
