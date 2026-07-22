//! FHIR R4/R5 adapter surface for Solum (stage 1).
//!
//! Placeholder crate: wire HL7 FHIR resources into jurisdiction-aware
//! encryption and audit policies. No clinical interpretation logic here —
//! see CONTRIBUTING.md (MDCG boundary).

#![forbid(unsafe_code)]

/// Stage marker for roadmap / capability reporting.
pub const STAGE: &str = "1-scaffold";

/// Placeholder handle for a future FHIR client / validator binding.
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
    fn scaffold_constructs() {
        let a = FhirAdapter::with_base_url("https://fhir.example.org/r4");
        assert!(a.base_url.is_some());
        assert_eq!(STAGE, "1-scaffold");
    }
}
