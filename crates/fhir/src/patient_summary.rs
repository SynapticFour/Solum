//! Minimal HL7 IPS–oriented Patient Summary binding (FHIR R4 document shape).
//!
//! This is **not** a full IPS Implementation Guide implementation and does not
//! claim IG conformance. It models the IPS document core used for Solum field
//! encryption category `patient_summary`: Patient demographics plus the three
//! required IPS Composition sections (Allergies, Medications, Problems).
//!
//! Reference: HL7 FHIR UV IPS STU 2 (Composition-uv-ips) — sections required
//! with either entries or `emptyReason` (ips-comp-1). Full profile validation,
//! mustSupport obligations, and terminology binding are stage-2 scope.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solum_crypto::{
    decrypt_field, encrypt_field, Crypt4ghKeyProvider, EncryptedField, FieldCategoryGate, KeyRef,
};
use thiserror::Error;

/// Solum field-category id for Patient Summary blobs (must match profile TOML).
pub const PATIENT_SUMMARY_CATEGORY: &str = "patient_summary";

/// IPS Composition.type coding (LOINC).
///
/// ANNAHME, bitte gegen aktuelle IPS-Spec prüfen: LOINC `60591-5`
/// ("Patient summary Document") is the long-standing IPS document type code
/// in HL7.FHIR.UV.IPS; confirm against the STU version you target.
const IPS_COMPOSITION_TYPE_LOINC: &str = "60591-5";

/// IPS section LOINC codes (required core sections).
///
/// ANNAHME, bitte gegen aktuelle IPS-Spec prüfen: classic IPS section codes
/// Allergies `48765-2`, Medications `10160-0`, Problems `11450-4`.
const SECTION_ALLERGIES_LOINC: &str = "48765-2";
const SECTION_MEDICATIONS_LOINC: &str = "10160-0";
const SECTION_PROBLEMS_LOINC: &str = "11450-4";

/// FHIR `Composition.section.emptyReason` when a required section has no entries.
///
/// ANNAHME, bitte gegen aktuelle IPS-Spec prüfen: IPS prefers asserting
/// “known absent” / “not known” inside clinical resources for the three
/// required sections; `emptyReason` remains allowed by ips-comp-1. We use
/// code-system `http://terminology.hl7.org/CodeSystem/list-empty-reason`
/// code `nilknown` for empty lists — not a clinical “no known allergy”
/// assertion.
const EMPTY_REASON_NILKNOWN: &str = "nilknown";

/// Minimal Patient Summary used as Solum’s encryptable clinical document unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientSummary {
    /// Composition.date (FHIR dateTime). Required for a document Bundle.
    pub date: String,
    /// FHIR Composition.author reference (mandatory in base FHIR R4, 1..*).
    /// Minimal slice: display text only, no full Reference datatype.
    ///
    /// ANNAHME, bitte gegen aktuelle FHIR-Spec prüfen: a Reference with only
    /// `display` (no `reference` URL) satisfies the Reference datatype and
    /// Composition.author cardinality; if a target profile forbids display-only
    /// authors, switch to an Organization entry + fullUrl reference.
    pub author_display: String,
    pub patient: PatientInfo,
    /// IPS Allergies and Intolerances section entries (may be empty).
    pub allergies: Vec<AllergyEntry>,
    /// IPS Medication Summary section entries (may be empty).
    pub medications: Vec<MedicationEntry>,
    /// IPS Problems section entries (may be empty).
    pub problems: Vec<ProblemEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientInfo {
    /// Local logical id used in Bundle fullUrl / Composition.subject.
    pub id: String,
    /// FHIR Patient.identifier (system + value). Minimal slice — not a full
    /// Identifier datatype (period, type, use omitted).
    pub identifier: Vec<Identifier>,
    /// FHIR Patient.name — HumanName family/given only.
    pub name: Vec<HumanName>,
    /// FHIR Patient.birthDate (`YYYY-MM-DD`), optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanName {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<String>,
}

/// Minimal allergy row (maps toward AllergyIntolerance, not full IPS profile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyEntry {
    pub id: String,
    /// Free-text or code display for the substance — ANNAHME: not bound to
    /// IPS value sets (e.g. SNOMED) in this stage-1 binding.
    pub substance_display: String,
}

/// Minimal medication row (maps toward MedicationStatement / MedicationRequest).
///
/// ANNAHME, bitte gegen aktuelle IPS-Spec prüfen: IPS allows MedicationStatement
/// **or** MedicationRequest in the Medication Summary section; we emit
/// MedicationStatement only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicationEntry {
    pub id: String,
    pub medication_display: String,
}

/// Minimal problem/condition row (maps toward Condition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemEntry {
    pub id: String,
    pub condition_display: String,
}

#[derive(Debug, Error)]
pub enum FhirError {
    #[error("JSON (de)serialisation failed: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Crypto(#[from] solum_crypto::CryptoError),
    #[error("decrypted payload is not a PatientSummary: {0}")]
    InvalidSummary(String),
}

/// Build a minimal FHIR R4 document Bundle for `summary`.
///
/// Produces `resourceType=Bundle`, `type=document`, with Composition first,
/// then Patient, then zero-or-more AllergyIntolerance / MedicationStatement /
/// Condition resources. Not IG-validated.
pub fn to_fhir_bundle(summary: &PatientSummary) -> Result<Value, FhirError> {
    let composition_id = "composition-ips";
    let patient_full_url = format!("urn:uuid:patient-{}", summary.patient.id);
    let composition_full_url = format!("urn:uuid:{composition_id}");

    let mut entries: Vec<Value> = Vec::new();
    let mut sections: Vec<Value> = Vec::new();

    // --- allergies ---
    let (allergy_section, allergy_entries) = section_with_resources(
        SECTION_ALLERGIES_LOINC,
        "Allergies and Intolerances",
        &summary.allergies,
        |a| {
            (
                format!("urn:uuid:allergy-{}", a.id),
                json!({
                    "resourceType": "AllergyIntolerance",
                    "id": a.id,
                    "patient": { "reference": patient_full_url },
                    "code": {
                        "text": a.substance_display
                    }
                }),
            )
        },
    );
    sections.push(allergy_section);
    entries.extend(allergy_entries);

    // --- medications ---
    let (med_section, med_entries) = section_with_resources(
        SECTION_MEDICATIONS_LOINC,
        "Medication Summary",
        &summary.medications,
        |m| {
            (
                format!("urn:uuid:medication-{}", m.id),
                json!({
                    "resourceType": "MedicationStatement",
                    "id": m.id,
                    "status": "unknown",
                    "medicationCodeableConcept": {
                        "text": m.medication_display
                    },
                    "subject": { "reference": patient_full_url }
                }),
            )
        },
    );
    sections.push(med_section);
    entries.extend(med_entries);

    // --- problems ---
    let (problem_section, problem_entries) = section_with_resources(
        SECTION_PROBLEMS_LOINC,
        "Problem List",
        &summary.problems,
        |p| {
            (
                format!("urn:uuid:problem-{}", p.id),
                json!({
                    "resourceType": "Condition",
                    "id": p.id,
                    "subject": { "reference": patient_full_url },
                    "code": {
                        "text": p.condition_display
                    }
                }),
            )
        },
    );
    sections.push(problem_section);
    entries.extend(problem_entries);

    let composition = json!({
        "resourceType": "Composition",
        "id": composition_id,
        "status": "final",
        "type": {
            "coding": [{
                "system": "http://loinc.org",
                "code": IPS_COMPOSITION_TYPE_LOINC,
                "display": "Patient summary Document"
            }]
        },
        // FHIR R4 Composition.author is 1..* Reference(…). Display-only is valid
        // Reference structure; see PatientSummary::author_display note.
        "author": [{ "display": summary.author_display }],
        "subject": { "reference": patient_full_url },
        "date": summary.date,
        "title": "International Patient Summary (Solum minimal)",
        "section": sections
    });

    let mut patient_resource = json!({
        "resourceType": "Patient",
        "id": summary.patient.id,
        "identifier": summary.patient.identifier.iter().map(|i| {
            let mut obj = json!({ "value": i.value });
            if let Some(sys) = &i.system {
                obj["system"] = json!(sys);
            }
            obj
        }).collect::<Vec<_>>(),
        "name": summary.patient.name.iter().map(|n| {
            json!({
                "family": n.family,
                "given": n.given
            })
        }).collect::<Vec<_>>()
    });
    if let Some(bd) = &summary.patient.birth_date {
        patient_resource["birthDate"] = json!(bd);
    }

    let mut bundle_entries = vec![
        json!({
            "fullUrl": composition_full_url,
            "resource": composition
        }),
        json!({
            "fullUrl": patient_full_url,
            "resource": patient_resource
        }),
    ];
    bundle_entries.extend(entries);

    // FHIR R4 Bundle invariants for type=document (hl7.org/fhir/R4/bundle.html):
    //   bdl-9  — identifier.system and identifier.value SHALL be present
    //   bdl-10 — timestamp SHALL be present
    // Synthetic stage-1 document id (not a persistent clinical OID assignment).
    let document_id_value = format!("urn:uuid:solum-ips-{}-{}", summary.patient.id, summary.date);

    Ok(json!({
        "resourceType": "Bundle",
        "type": "document",
        "identifier": {
            "system": "urn:ietf:rfc:3986",
            "value": document_id_value
        },
        "timestamp": summary.date,
        "entry": bundle_entries
    }))
}

fn section_with_resources<T, F>(
    loinc: &str,
    title: &str,
    items: &[T],
    to_resource: F,
) -> (Value, Vec<Value>)
where
    F: Fn(&T) -> (String, Value),
{
    if items.is_empty() {
        let section = json!({
            "title": title,
            "code": {
                "coding": [{
                    "system": "http://loinc.org",
                    "code": loinc
                }]
            },
            "emptyReason": {
                "coding": [{
                    "system": "http://terminology.hl7.org/CodeSystem/list-empty-reason",
                    "code": EMPTY_REASON_NILKNOWN
                }]
            },
            // ANNAHME: IPS requires section.text when the section is present;
            // we emit a minimal narrative placeholder, not a rendered clinical text.
            "text": {
                "status": "generated",
                "div": format!("<div xmlns=\"http://www.w3.org/1999/xhtml\">{title}: no information</div>")
            }
        });
        (section, Vec::new())
    } else {
        let mut section_entries = Vec::new();
        let mut bundle_entries = Vec::new();
        for item in items {
            let (full_url, resource) = to_resource(item);
            section_entries.push(json!({ "reference": full_url }));
            bundle_entries.push(json!({
                "fullUrl": full_url,
                "resource": resource
            }));
        }
        let section = json!({
            "title": title,
            "code": {
                "coding": [{
                    "system": "http://loinc.org",
                    "code": loinc
                }]
            },
            "entry": section_entries,
            "text": {
                "status": "generated",
                "div": format!("<div xmlns=\"http://www.w3.org/1999/xhtml\">{title}</div>")
            }
        });
        (section, bundle_entries)
    }
}

/// Encrypt a [`PatientSummary`] as Crypt4GH category `patient_summary`.
///
/// Serialises the Solum summary model (not the FHIR Bundle) so decrypt yields
/// the same typed structure. Callers that need Bundle shape use
/// [`to_fhir_bundle`] before or after decryption.
pub fn encrypt_patient_summary(
    gate: &FieldCategoryGate<'_>,
    summary: &PatientSummary,
    key_ref: &KeyRef,
    provider: &impl Crypt4ghKeyProvider,
) -> Result<EncryptedField, FhirError> {
    let plaintext = serde_json::to_vec(summary)?;
    Ok(encrypt_field(
        gate,
        provider,
        PATIENT_SUMMARY_CATEGORY,
        &plaintext,
        key_ref,
    )?)
}

/// Decrypt a field previously produced by [`encrypt_patient_summary`].
pub fn decrypt_patient_summary(
    provider: &impl Crypt4ghKeyProvider,
    field: &EncryptedField,
    key_ref: &KeyRef,
) -> Result<PatientSummary, FhirError> {
    if field.category != PATIENT_SUMMARY_CATEGORY {
        return Err(FhirError::InvalidSummary(format!(
            "expected category '{PATIENT_SUMMARY_CATEGORY}', got '{}'",
            field.category
        )));
    }
    let bytes = decrypt_field(provider, field, key_ref)?;
    serde_json::from_slice(&bytes).map_err(|e| FhirError::InvalidSummary(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solum_crypto::{EphemeralTestKeyProvider, FieldCategoryGate, KeyRef};
    use solum_profiles::load_profile;
    use std::path::PathBuf;

    fn sample_summary() -> PatientSummary {
        PatientSummary {
            date: "2026-07-26T10:00:00Z".into(),
            author_display: "Solum Compliance Layer (stage-1, non-clinical)".into(),
            patient: PatientInfo {
                id: "pat-1".into(),
                identifier: vec![Identifier {
                    system: Some("urn:oid:2.16.840.1.113883.2.4.6.3".into()),
                    value: "999999001".into(),
                }],
                name: vec![HumanName {
                    family: Some("Doe".into()),
                    given: vec!["Jane".into()],
                }],
                birth_date: Some("1980-05-01".into()),
            },
            allergies: vec![AllergyEntry {
                id: "alg-1".into(),
                substance_display: "Penicillin".into(),
            }],
            medications: vec![MedicationEntry {
                id: "med-1".into(),
                medication_display: "Lisinopril 10mg".into(),
            }],
            problems: vec![ProblemEntry {
                id: "prb-1".into(),
                condition_display: "Hypertension".into(),
            }],
        }
    }

    fn profiles_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/profiles")
    }

    #[test]
    fn to_fhir_bundle_has_document_composition_and_patient() {
        let bundle = to_fhir_bundle(&sample_summary()).expect("bundle");
        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "document");
        let entries = bundle["entry"].as_array().expect("entry array");
        assert!(entries.len() >= 2);

        let composition = &entries[0]["resource"];
        assert_eq!(composition["resourceType"], "Composition");
        assert_eq!(
            composition["type"]["coding"][0]["code"],
            IPS_COMPOSITION_TYPE_LOINC
        );
        let authors = composition["author"]
            .as_array()
            .expect("Composition.author must be present (FHIR R4 1..*)");
        assert!(!authors.is_empty(), "Composition.author must be non-empty");
        assert_eq!(
            authors[0]["display"],
            "Solum Compliance Layer (stage-1, non-clinical)"
        );

        assert!(
            bundle["identifier"]["system"].as_str().is_some(),
            "document Bundle.identifier.system required (bdl-9)"
        );
        assert!(
            bundle["identifier"]["value"].as_str().is_some(),
            "document Bundle.identifier.value required (bdl-9)"
        );
        assert!(
            bundle["timestamp"].as_str().is_some(),
            "document Bundle.timestamp required (bdl-10)"
        );

        let patient = &entries[1]["resource"];
        assert_eq!(patient["resourceType"], "Patient");
        assert_eq!(patient["id"], "pat-1");
        assert_eq!(patient["birthDate"], "1980-05-01");

        let types: Vec<&str> = entries
            .iter()
            .filter_map(|e| e["resource"]["resourceType"].as_str())
            .collect();
        assert!(types.contains(&"AllergyIntolerance"));
        assert!(types.contains(&"MedicationStatement"));
        assert!(types.contains(&"Condition"));
    }

    #[test]
    fn empty_clinical_lists_are_valid_and_emit_empty_reason() {
        let mut summary = sample_summary();
        summary.allergies.clear();
        summary.medications.clear();
        summary.problems.clear();

        let bundle = to_fhir_bundle(&summary).expect("bundle with empty sections");
        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "document");

        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(
            entries.len(),
            2,
            "only Composition + Patient when lists empty"
        );

        let sections = entries[0]["resource"]["section"].as_array().unwrap();
        assert_eq!(sections.len(), 3);
        for section in sections {
            assert!(
                section.get("emptyReason").is_some(),
                "empty section should set emptyReason: {section}"
            );
            assert!(
                section.get("entry").is_none(),
                "empty section must not list entries"
            );
        }
    }

    #[test]
    fn encrypt_decrypt_patient_summary_round_trip() {
        let profile = load_profile(profiles_dir().join("eu-ehds.toml")).expect("eu-ehds");
        let gate = FieldCategoryGate::new(&profile.encryption.required_field_categories);
        let key_ref = KeyRef::new("ephemeral/ips-1");
        let mut provider = EphemeralTestKeyProvider::new();
        provider.generate_test_keypair(key_ref.clone()).unwrap();

        let summary = sample_summary();
        let enc = encrypt_patient_summary(&gate, &summary, &key_ref, &provider).unwrap();
        assert_eq!(enc.category, PATIENT_SUMMARY_CATEGORY);
        let out = decrypt_patient_summary(&provider, &enc, &key_ref).unwrap();
        assert_eq!(out, summary);
    }

    #[test]
    fn decrypt_patient_summary_wrong_key_fails() {
        let profile = load_profile(profiles_dir().join("eu-ehds.toml")).expect("eu-ehds");
        let gate = FieldCategoryGate::new(&profile.encryption.required_field_categories);
        let key_a = KeyRef::new("ephemeral/a");
        let key_b = KeyRef::new("ephemeral/b");
        let mut provider = EphemeralTestKeyProvider::new();
        provider.generate_test_keypair(key_a.clone()).unwrap();
        provider.generate_test_keypair(key_b.clone()).unwrap();

        let enc = encrypt_patient_summary(&gate, &sample_summary(), &key_a, &provider).unwrap();
        let err = decrypt_patient_summary(&provider, &enc, &key_b).unwrap_err();
        assert!(matches!(err, FhirError::Crypto(_)));
    }

    #[test]
    fn patient_summary_category_required_in_eu_and_kenya_profiles() {
        let eu = load_profile(profiles_dir().join("eu-ehds.toml")).expect("eu-ehds.toml");
        let ke = load_profile(profiles_dir().join("kenya-dpa.toml")).expect("kenya-dpa.toml");
        assert!(
            eu.encryption
                .required_field_categories
                .iter()
                .any(|c| c == PATIENT_SUMMARY_CATEGORY),
            "eu-ehds.toml must list patient_summary"
        );
        assert!(
            ke.encryption
                .required_field_categories
                .iter()
                .any(|c| c == PATIENT_SUMMARY_CATEGORY),
            "kenya-dpa.toml must list patient_summary"
        );
    }
}
