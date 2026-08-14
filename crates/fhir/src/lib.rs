//! FHIR R4 adapter surface for Solum (stage 1).
//!
//! Stage-1 binding starts with a minimal HL7 International Patient Summary
//! (IPS)–oriented [`PatientSummary`] document unit, encryptable as field
//! category `patient_summary` via Crypt4GH. This is not a full FHIR client /
//! validator and performs no clinical interpretation — see CONTRIBUTING.md
//! (MDCG boundary).

#![forbid(unsafe_code)]

mod patient_summary;

pub use patient_summary::{
    decrypt_patient_summary, encrypt_patient_summary, to_fhir_bundle, AllergyEntry, FhirError,
    HumanName, Identifier, MedicationEntry, PatientInfo, PatientSummary, ProblemEntry,
    PATIENT_SUMMARY_CATEGORY,
};

/// IPS-aligned resource types allowed on the H3.1 façade and H3.2 importer.
pub const ALLOWED_FHIR_RESOURCE_TYPES: &[&str] = &[
    "Bundle",
    "Composition",
    "Patient",
    "AllergyIntolerance",
    "MedicationStatement",
    "Condition",
];

/// Whether `resource_type` is in [`ALLOWED_FHIR_RESOURCE_TYPES`].
pub fn fhir_resource_type_allowed(resource_type: &str) -> bool {
    ALLOWED_FHIR_RESOURCE_TYPES.contains(&resource_type)
}

/// Stage marker for roadmap / capability reporting.
///
/// `1-patient-summary` = IPS-oriented Patient Summary model + Bundle export +
/// Crypt4GH encrypt/decrypt helpers. Full IPS IG conformance remains open.
pub const STAGE: &str = "1-patient-summary";

/// Optional FHIR base URL handle (config only — no HTTP client in this crate).
#[derive(Debug, Default, Clone)]
#[deprecated(
    since = "0.1.0",
    note = "config handle only; interchange lives in PatientSummary / sidecar FhirStore"
)]
pub struct FhirAdapter {
    pub base_url: Option<String>,
}

#[allow(deprecated)]
impl FhirAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: Some(base_url.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn adapter_constructs_and_stage_reports_patient_summary() {
        let a = FhirAdapter::with_base_url("https://fhir.example.org/r4");
        assert!(a.base_url.is_some());
        assert_eq!(STAGE, "1-patient-summary");
    }
}
