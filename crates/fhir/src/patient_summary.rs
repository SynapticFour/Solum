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

/// Provisional Composition.extension URL for [`PatientSummary::mii_validation_ref`].
///
/// ANNAHME: not a standardised / jointly agreed FHIR StructureDefinition URL —
/// Solum-internal placeholder until Ferrum/Solum align on a real extension
/// (or drop the extension in favour of Bundle.meta / Provenance).
const MII_VALIDATION_REF_EXTENSION_URL: &str =
    "https://synapticfour.com/fhir/StructureDefinition/solum-mii-validation-ref";

/// IPS Composition.type coding (LOINC).
///
/// LOINC `60591-5` official display is **Patient Summary** (validator locale
/// may reject legacy "Patient summary Document").
const IPS_COMPOSITION_TYPE_LOINC: &str = "60591-5";
const IPS_COMPOSITION_TYPE_DISPLAY: &str = "Patient Summary";

/// Stable UUID namespace for Solum document Bundle fullUrls (UUID v5).
/// Not a public OID assignment — deterministic so re-export is stable for a given logical id.
const SOLUM_FHIR_UUID_NS: uuid::Uuid = uuid::Uuid::from_bytes([
    0x73, 0x6f, 0x6c, 0x75, 0x6d, 0x2d, 0x66, 0x68, 0x69, 0x72, 0x2d, 0x6e, 0x73, 0x2d, 0x31, 0x00,
]);

/// `urn:uuid:<v5>` for Bundle.entry.fullUrl and cross-references.
fn urn_uuid_for(name: &str) -> String {
    format!(
        "urn:uuid:{}",
        uuid::Uuid::new_v5(&SOLUM_FHIR_UUID_NS, name.as_bytes())
    )
}

fn generated_narrative(plain: &str) -> Value {
    let escaped = plain
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    json!({
        "status": "generated",
        "div": format!("<div xmlns=\"http://www.w3.org/1999/xhtml\">{escaped}</div>")
    })
}
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
    /// FHIR Composition.author — display label for an Organization entry in the
    /// document Bundle. Stage‑1 emits Organization + `reference` fullUrl (not
    /// display-only).
    pub author_display: String,
    pub patient: PatientInfo,
    /// IPS Allergies and Intolerances section entries (may be empty).
    pub allergies: Vec<AllergyEntry>,
    /// IPS Medication Summary section entries (may be empty).
    pub medications: Vec<MedicationEntry>,
    /// IPS Problems section entries (may be empty).
    pub problems: Vec<ProblemEntry>,
    /// Optional reference to a prior structural FHIR validation report
    /// (e.g. from Ferrum's `ferrum-mii-connect`), when running in
    /// Ferrum-companion mode. Solum does not perform or replicate that
    /// validation itself — this is a passthrough marker only. `None` in
    /// standalone mode (no Ferrum involved) or when no such report exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mii_validation_ref: Option<String>,
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
    let patient_full_url = urn_uuid_for(&format!("patient:{}", summary.patient.id));
    let composition_full_url = urn_uuid_for(composition_id);

    let mut entries: Vec<Value> = Vec::new();
    let mut sections: Vec<Value> = Vec::new();

    // --- allergies ---
    let (allergy_section, allergy_entries) = section_with_resources(
        SECTION_ALLERGIES_LOINC,
        "Allergies and Intolerances",
        &summary.allergies,
        |a| {
            let full_url = urn_uuid_for(&format!("allergy:{}", a.id));
            (
                full_url,
                json!({
                    "resourceType": "AllergyIntolerance",
                    "id": a.id,
                    "text": generated_narrative(&format!("Allergy: {}", a.substance_display)),
                    // ait-1: clinicalStatus required when verificationStatus is absent /
                    // not entered-in-error.
                    "clinicalStatus": {
                        "coding": [{
                            "system": "http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical",
                            "code": "active",
                            "display": "Active"
                        }]
                    },
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
            let full_url = urn_uuid_for(&format!("medication:{}", m.id));
            (
                full_url,
                json!({
                    "resourceType": "MedicationStatement",
                    "id": m.id,
                    "text": generated_narrative(&format!("Medication: {}", m.medication_display)),
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
            let full_url = urn_uuid_for(&format!("problem:{}", p.id));
            (
                full_url,
                json!({
                    "resourceType": "Condition",
                    "id": p.id,
                    "text": generated_narrative(&format!("Condition: {}", p.condition_display)),
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

    let author_full_url = urn_uuid_for(&format!("author:{}", summary.patient.id));
    let mut composition = json!({
        "resourceType": "Composition",
        "id": composition_id,
        "text": generated_narrative(&summary.author_display),
        "status": "final",
        "type": {
            "coding": [{
                "system": "http://loinc.org",
                "code": IPS_COMPOSITION_TYPE_LOINC,
                "display": IPS_COMPOSITION_TYPE_DISPLAY
            }]
        },
        "author": [{
            "reference": author_full_url,
            "display": summary.author_display
        }],
        "subject": { "reference": patient_full_url },
        "date": summary.date,
        "title": "International Patient Summary (Solum minimal)",
        "section": sections
    });
    if let Some(mii_ref) = &summary.mii_validation_ref {
        composition["extension"] = json!([{
            "url": MII_VALIDATION_REF_EXTENSION_URL,
            "valueString": mii_ref
        }]);
    }

    let mut patient_resource = json!({
        "resourceType": "Patient",
        "id": summary.patient.id,
        "text": generated_narrative(&format!("Patient {}", summary.patient.id)),
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

    let author_resource = json!({
        "resourceType": "Organization",
        "id": format!("org-{}", summary.patient.id),
        "text": generated_narrative(&summary.author_display),
        "name": summary.author_display
    });

    let mut bundle_entries = vec![
        json!({
            "fullUrl": composition_full_url,
            "resource": composition
        }),
        json!({
            "fullUrl": patient_full_url,
            "resource": patient_resource
        }),
        json!({
            "fullUrl": author_full_url,
            "resource": author_resource
        }),
    ];
    bundle_entries.extend(entries);

    // FHIR R4 Bundle invariants for type=document (hl7.org/fhir/R4/bundle.html):
    //   bdl-9  — identifier.system and identifier.value SHALL be present
    //   bdl-10 — timestamp SHALL be present
    let document_id_value =
        urn_uuid_for(&format!("document:{}:{}", summary.patient.id, summary.date));

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
///
/// Prefer [`solum_core::Deployment::encrypt_patient_summary_as`] for durable
/// `data.encrypt` audit. These crate-local helpers remain for unit tests and
/// callers that already own audit elsewhere.
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
            mii_validation_ref: None,
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
        assert_eq!(
            composition["type"]["coding"][0]["display"],
            IPS_COMPOSITION_TYPE_DISPLAY
        );
        let authors = composition["author"]
            .as_array()
            .expect("Composition.author must be present (FHIR R4 1..*)");
        assert!(!authors.is_empty(), "Composition.author must be non-empty");
        assert_eq!(
            authors[0]["display"],
            "Solum Compliance Layer (stage-1, non-clinical)"
        );
        let author_ref = authors[0]["reference"]
            .as_str()
            .expect("Composition.author.reference");
        assert!(
            author_ref.starts_with("urn:uuid:"),
            "author must reference Organization fullUrl: {author_ref}"
        );
        let org = entries
            .iter()
            .find(|e| e["resource"]["resourceType"] == "Organization")
            .expect("Organization author entry");
        assert_eq!(org["fullUrl"], author_ref);

        for entry in entries {
            let full = entry["fullUrl"].as_str().expect("fullUrl");
            assert!(
                full.starts_with("urn:uuid:"),
                "fullUrl must be urn:uuid: {full}"
            );
            let uuid_part = full.strip_prefix("urn:uuid:").unwrap();
            assert!(
                uuid::Uuid::parse_str(uuid_part).is_ok(),
                "fullUrl must contain a valid UUID: {full}"
            );
            assert!(
                entry["resource"]["text"]["div"].as_str().is_some(),
                "DomainResource should carry generated narrative"
            );
        }

        let allergy = entries
            .iter()
            .find(|e| e["resource"]["resourceType"] == "AllergyIntolerance")
            .expect("allergy entry");
        assert!(
            allergy["resource"]["clinicalStatus"]["coding"][0]["code"] == "active",
            "ait-1: clinicalStatus required"
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
            3,
            "Composition + Patient + Organization author when lists empty"
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

    #[test]
    fn mii_validation_ref_none_serde_omits_field_and_roundtrips() {
        let summary = sample_summary();
        assert_eq!(summary.mii_validation_ref, None);
        let json = serde_json::to_string(&summary).unwrap();
        assert!(
            !json.contains("miiValidationRef"),
            "None must be skipped for backward-compatible JSON: {json}"
        );
        let back: PatientSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, summary);

        // Legacy payload without the field deserialises to None.
        let legacy = r#"{
            "date":"2026-07-26T10:00:00Z",
            "authorDisplay":"Solum Compliance Layer (stage-1, non-clinical)",
            "patient":{
                "id":"pat-1",
                "identifier":[{"system":"urn:oid:2.16.840.1.113883.2.4.6.3","value":"999999001"}],
                "name":[{"family":"Doe","given":["Jane"]}],
                "birthDate":"1980-05-01"
            },
            "allergies":[{"id":"alg-1","substanceDisplay":"Penicillin"}],
            "medications":[{"id":"med-1","medicationDisplay":"Lisinopril 10mg"}],
            "problems":[{"id":"prb-1","conditionDisplay":"Hypertension"}]
        }"#;
        let from_legacy: PatientSummary = serde_json::from_str(legacy).unwrap();
        assert_eq!(from_legacy.mii_validation_ref, None);
        assert_eq!(from_legacy, summary);
    }

    #[test]
    fn mii_validation_ref_appears_on_composition_extension() {
        let mut summary = sample_summary();
        summary.mii_validation_ref = Some("mii-report:fixture-42".into());
        let bundle = to_fhir_bundle(&summary).expect("bundle");
        let composition = &bundle["entry"][0]["resource"];
        let ext = composition["extension"]
            .as_array()
            .expect("Composition.extension present when mii_validation_ref set");
        assert_eq!(ext.len(), 1);
        assert_eq!(ext[0]["url"], MII_VALIDATION_REF_EXTENSION_URL);
        assert_eq!(ext[0]["valueString"], "mii-report:fixture-42");
    }
}
