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

/// Stage marker for roadmap / capability reporting.
///
/// `1-patient-summary` = IPS-oriented Patient Summary model + Bundle export +
/// Crypt4GH encrypt/decrypt helpers. Full IPS IG conformance remains open.
pub const STAGE: &str = "1-patient-summary";

/// Optional FHIR base URL handle for a future client binding.
#[derive(Debug, Default, Clone)]
pub struct FhirAdapter {
    pub base_url: Option<String>,
}

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
    fn adapter_constructs_and_stage_reports_patient_summary() {
        let a = FhirAdapter::with_base_url("https://fhir.example.org/r4");
        assert!(a.base_url.is_some());
        assert_eq!(STAGE, "1-patient-summary");
    }
}
