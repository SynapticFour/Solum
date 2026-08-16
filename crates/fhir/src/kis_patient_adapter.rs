//! KIS / ISiK-shaped Patient adapter (not a hospital EHR, not IG-validated).
//!
//! Maps Solum [`PatientInfo`] toward the identifier/name/birthDate slice a
//! German KIS typically expects (KVID-10 system URL when the operator supplies
//! that identifier). Does **not** claim ISiK, gematik, or TI conformance.

use serde_json::{json, Value};

use crate::patient_summary::PatientInfo;

/// GKV KVID-10 identifier system (public HL7 DE convention).
pub const DE_KVID10_SYSTEM: &str = "http://fhir.de/sid/gkv/kvid-10";

/// Local fallback when the operator has not stamped a DE identifier system.
pub const SOLUM_LOCAL_PATIENT_SYSTEM: &str = "https://synapticfour.com/fhir/sid/local-patient";

/// Adapter tag so receivers can see this is a Solum mapping, not an IG instance.
pub const KIS_ADAPTER_TAG_SYSTEM: &str = "https://synapticfour.com/fhir/CodeSystem/solum-adapter";
pub const KIS_ADAPTER_TAG_CODE: &str = "kis-patient-v0";

/// FHIR R4 Patient JSON for exchange with an existing KIS.
///
/// Not ISiK-validated. No SMC-B / TI auth. Operators replace the local
/// identifier system with KVID-10 (or the site's MPI) as appropriate.
pub fn to_kis_patient_adapter(patient: &PatientInfo) -> Value {
    let identifiers: Vec<Value> = if patient.identifier.is_empty() {
        vec![json!({
            "system": SOLUM_LOCAL_PATIENT_SYSTEM,
            "value": patient.id,
        })]
    } else {
        patient
            .identifier
            .iter()
            .map(|id| {
                json!({
                    "system": id.system.as_deref().unwrap_or(SOLUM_LOCAL_PATIENT_SYSTEM),
                    "value": id.value,
                })
            })
            .collect()
    };

    let names: Vec<Value> = patient
        .name
        .iter()
        .map(|n| {
            let mut obj = serde_json::Map::new();
            if let Some(family) = &n.family {
                obj.insert("family".into(), Value::String(family.clone()));
            }
            if !n.given.is_empty() {
                obj.insert(
                    "given".into(),
                    Value::Array(n.given.iter().cloned().map(Value::String).collect()),
                );
            }
            Value::Object(obj)
        })
        .collect();

    let mut resource = json!({
        "resourceType": "Patient",
        "id": patient.id,
        "meta": {
            "tag": [{
                "system": KIS_ADAPTER_TAG_SYSTEM,
                "code": KIS_ADAPTER_TAG_CODE,
                "display": "KIS Patient adapter (not ISiK-validated)"
            }]
        },
        "identifier": identifiers,
        "name": names,
    });
    if let Some(bd) = &patient.birth_date {
        resource
            .as_object_mut()
            .expect("patient object")
            .insert("birthDate".into(), Value::String(bd.clone()));
    }
    resource
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patient_summary::{HumanName, Identifier};

    #[test]
    fn stamps_kvid_when_operator_supplies_system() {
        let patient = PatientInfo {
            id: "p-1".into(),
            identifier: vec![Identifier {
                system: Some(DE_KVID10_SYSTEM.into()),
                value: "X123456789".into(),
            }],
            name: vec![HumanName {
                family: Some("Muster".into()),
                given: vec!["Erika".into()],
            }],
            birth_date: Some("1970-01-01".into()),
        };
        let json = to_kis_patient_adapter(&patient);
        assert_eq!(json["resourceType"], "Patient");
        assert_eq!(json["identifier"][0]["system"], DE_KVID10_SYSTEM);
        assert_eq!(json["meta"]["tag"][0]["code"], KIS_ADAPTER_TAG_CODE);
        assert!(json.get("meta").unwrap().get("profile").is_none());
    }

    #[test]
    fn local_system_when_identifier_missing() {
        let patient = PatientInfo {
            id: "p-2".into(),
            identifier: vec![],
            name: vec![],
            birth_date: None,
        };
        let json = to_kis_patient_adapter(&patient);
        assert_eq!(json["identifier"][0]["system"], SOLUM_LOCAL_PATIENT_SYSTEM);
        assert_eq!(json["identifier"][0]["value"], "p-2");
    }
}
