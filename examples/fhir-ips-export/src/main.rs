//! Emit one IPS-oriented document Bundle JSON for offline validator probes.
//! Synthetic data only — not real PHI, not an ISiK/TI claim.

use solum_fhir::{
    to_fhir_bundle, AllergyEntry, HumanName, Identifier, MedicationEntry, PatientInfo,
    PatientSummary, ProblemEntry,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn sample() -> PatientSummary {
    PatientSummary {
        date: "2026-08-11T09:00:00Z".into(),
        author_display: "Solum Compliance Layer (stage-1, non-clinical)".into(),
        patient: PatientInfo {
            id: "we-nordlicht-1".into(),
            identifier: vec![Identifier {
                system: Some("urn:oid:2.16.840.1.113883.2.4.6.3".into()),
                value: "999999001".into(),
            }],
            name: vec![HumanName {
                family: Some("Muster".into()),
                given: vec!["Erika".into()],
            }],
            birth_date: Some("1975-03-15".into()),
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

fn main() {
    let out: PathBuf = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("examples/fhir-ips-export/out/patient-summary-bundle.json")
    });

    if let Some(parent) = out.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("mkdir {}: {e}", parent.display());
            process::exit(1);
        }
    }

    let bundle = match to_fhir_bundle(&sample()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("to_fhir_bundle: {e}");
            process::exit(1);
        }
    };

    let pretty = match serde_json::to_string_pretty(&bundle) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serialize: {e}");
            process::exit(1);
        }
    };

    if let Err(e) = fs::write(&out, pretty) {
        eprintln!("write {}: {e}", out.display());
        process::exit(1);
    }
    println!("wrote {}", out.display());
}
