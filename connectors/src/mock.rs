//! In-memory connector with a fixed field table. Not a hospital system.

use crate::{
    ConnectorError, ConsentAlignment, ErrorDisposition, KisConnector, KisConsentClaim, MappedField,
    SidecarConsent, SourceField,
};

/// Reference stub. The field table is generic names, not a vendor export.
#[derive(Debug, Default, Clone, Copy)]
pub struct MockKisConnector;

impl MockKisConnector {
    pub const ID: &'static str = "mock";

    fn solum_field(source_name: &str) -> Option<&'static str> {
        match source_name {
            "patient_id" => Some("subject_id"),
            "given_name" => Some("name.given"),
            "family_name" => Some("name.family"),
            _ => None,
        }
    }
}

impl KisConnector for MockKisConnector {
    fn reconcile_consent(
        &self,
        kis: &KisConsentClaim,
        sidecar: &SidecarConsent,
    ) -> Result<ConsentAlignment, ConnectorError> {
        if kis.subject_id.is_empty() || sidecar.subject_id.is_empty() {
            return Err(ConnectorError::InvalidInput("subject_id is empty"));
        }
        if kis.purpose.is_empty() || sidecar.purpose.is_empty() {
            return Err(ConnectorError::InvalidInput("purpose is empty"));
        }
        if kis.subject_id != sidecar.subject_id || kis.purpose != sidecar.purpose {
            return Err(ConnectorError::InvalidInput(
                "KIS claim and sidecar record are not the same subject and purpose",
            ));
        }
        let Some(kis_granted) = kis.granted else {
            return Err(ConnectorError::ConsentUnstated {
                side: "the KIS export",
            });
        };
        let Some(sidecar_granted) = sidecar.granted else {
            return Err(ConnectorError::ConsentUnstated {
                side: "the sidecar record",
            });
        };
        if kis_granted != sidecar_granted {
            return Err(ConnectorError::ConsentMismatch {
                kis_granted,
                sidecar_granted,
            });
        }
        Ok(ConsentAlignment {
            granted: kis_granted,
        })
    }

    fn map_fields(&self, fields: &[SourceField]) -> Result<Vec<MappedField>, ConnectorError> {
        let mut mapped = Vec::with_capacity(fields.len());
        for field in fields {
            if field.name.is_empty() {
                return Err(ConnectorError::InvalidInput("source field name is empty"));
            }
            if field.value.trim().is_empty() {
                return Err(ConnectorError::EmptyValue {
                    name: field.name.clone(),
                });
            }
            let Some(solum_field) = Self::solum_field(&field.name) else {
                return Err(ConnectorError::UnmappedField {
                    name: field.name.clone(),
                });
            };
            mapped.push(MappedField {
                source_name: field.name.clone(),
                solum_field: solum_field.to_string(),
                value: field.value.clone(),
            });
        }
        Ok(mapped)
    }

    fn classify_error(&self, error: &ConnectorError) -> ErrorDisposition {
        let reason = match error {
            ConnectorError::InvalidInput(_) => {
                "stop: the input does not name one subject and purpose"
            }
            ConnectorError::ConsentUnstated { .. } => {
                "stop: a missing consent statement is not a decision"
            }
            ConnectorError::ConsentMismatch { .. } => "stop: the KIS and the sidecar disagree",
            ConnectorError::UnmappedField { .. } => "stop: an unknown field is not dropped",
            ConnectorError::EmptyValue { .. } => "stop: an empty value is not present",
        };
        ErrorDisposition { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(granted: Option<bool>) -> KisConsentClaim {
        KisConsentClaim {
            subject_id: "subject-1".into(),
            purpose: "care".into(),
            granted,
        }
    }

    fn sidecar(granted: Option<bool>) -> SidecarConsent {
        SidecarConsent {
            subject_id: "subject-1".into(),
            purpose: "care".into(),
            granted,
        }
    }

    #[test]
    fn mock_id_is_not_a_named_kis() {
        assert_eq!(MockKisConnector::ID, "mock");
    }

    #[test]
    fn consent_match_is_the_only_ok() {
        let connector = MockKisConnector;
        let aligned = connector
            .reconcile_consent(&claim(Some(true)), &sidecar(Some(true)))
            .unwrap();
        assert!(aligned.granted);
        let refused = connector
            .reconcile_consent(&claim(Some(false)), &sidecar(Some(false)))
            .unwrap();
        assert!(!refused.granted);
    }

    #[test]
    fn missing_or_disagreeing_consent_stops() {
        let connector = MockKisConnector;
        let unstated = connector
            .reconcile_consent(&claim(None), &sidecar(Some(true)))
            .unwrap_err();
        assert!(matches!(
            unstated,
            ConnectorError::ConsentUnstated {
                side: "the KIS export"
            }
        ));
        assert!(connector
            .classify_error(&unstated)
            .reason
            .starts_with("stop:"));

        let absent = connector
            .reconcile_consent(&claim(Some(true)), &sidecar(None))
            .unwrap_err();
        assert!(matches!(absent, ConnectorError::ConsentUnstated { .. }));

        let mismatch = connector
            .reconcile_consent(&claim(Some(true)), &sidecar(Some(false)))
            .unwrap_err();
        assert_eq!(
            mismatch,
            ConnectorError::ConsentMismatch {
                kis_granted: true,
                sidecar_granted: false,
            }
        );
        assert!(connector
            .classify_error(&mismatch)
            .reason
            .contains("disagree"));
    }

    #[test]
    fn maps_known_fields_and_rejects_the_unknown() {
        let connector = MockKisConnector;
        let mapped = connector
            .map_fields(&[
                SourceField {
                    name: "patient_id".into(),
                    value: "subject-1".into(),
                },
                SourceField {
                    name: "family_name".into(),
                    value: "Muster".into(),
                },
            ])
            .unwrap();
        assert_eq!(mapped[0].solum_field, "subject_id");
        assert_eq!(mapped[1].solum_field, "name.family");

        let unknown = connector
            .map_fields(&[SourceField {
                name: "vendor_only_column".into(),
                value: "x".into(),
            }])
            .unwrap_err();
        assert!(matches!(unknown, ConnectorError::UnmappedField { .. }));
        assert!(connector
            .classify_error(&unknown)
            .reason
            .contains("unknown"));

        let empty = connector
            .map_fields(&[SourceField {
                name: "given_name".into(),
                value: "  ".into(),
            }])
            .unwrap_err();
        assert!(matches!(empty, ConnectorError::EmptyValue { .. }));
    }
}
