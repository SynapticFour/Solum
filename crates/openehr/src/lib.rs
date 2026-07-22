//! openEHR adapter surface for Solum (stage 2 scaffold).
//!
//! Intentionally minimal: stage 1 focuses on FHIR. openEHR composition /
//! archetype binding will expand here without changing the jurisdiction
//! profile schema.

#![forbid(unsafe_code)]

/// Stage marker for roadmap / capability reporting.
pub const STAGE: &str = "2-scaffold";

/// Placeholder handle for a future openEHR CDR / AQL binding.
#[derive(Debug, Default, Clone)]
pub struct OpenEhrAdapter {
    pub cdr_url: Option<String>,
}

impl OpenEhrAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_constructs() {
        assert!(OpenEhrAdapter::new().cdr_url.is_none());
        assert_eq!(STAGE, "2-scaffold");
    }
}
