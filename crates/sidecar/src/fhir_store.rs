//! File-backed FHIR resource store for H3.1 façade (JSONL).

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// IPS-aligned resource types allowed on the H3.1 façade.
pub const ALLOWED_FHIR_TYPES: &[&str] = &[
    "Bundle",
    "Composition",
    "Patient",
    "AllergyIntolerance",
    "MedicationStatement",
    "Condition",
];

pub fn fhir_type_allowed(resource_type: &str) -> bool {
    ALLOWED_FHIR_TYPES.contains(&resource_type)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFhirResource {
    pub resource_type: String,
    pub id: String,
    pub resource: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ehr_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_uid: Option<String>,
}

#[derive(Debug)]
pub struct FhirStore {
    path: PathBuf,
}

impl FhirStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("fhir store mkdir {}: {e}", parent.display()))?;
        }
        if !path.exists() {
            File::create(&path)
                .map_err(|e| format!("fhir store create {}: {e}", path.display()))?;
        }
        Ok(Self { path })
    }

    pub fn upsert(&self, entry: &StoredFhirResource) -> Result<(), String> {
        let mut keep: Vec<StoredFhirResource> = self.read_all()?;
        keep.retain(|e| !(e.resource_type == entry.resource_type && e.id == entry.id));
        keep.push(entry.clone());
        self.rewrite_all(&keep)
    }

    pub fn get(&self, resource_type: &str, id: &str) -> Result<Option<StoredFhirResource>, String> {
        Ok(self
            .read_all()?
            .into_iter()
            .find(|e| e.resource_type == resource_type && e.id == id))
    }

    fn read_all(&self) -> Result<Vec<StoredFhirResource>, String> {
        let file = File::open(&self.path)
            .map_err(|e| format!("fhir store open {}: {e}", self.path.display()))?;
        let mut out = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("fhir store read: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(&line).map_err(|e| format!("fhir store parse: {e}"))?);
        }
        Ok(out)
    }

    fn rewrite_all(&self, entries: &[StoredFhirResource]) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| format!("fhir store rewrite {}: {e}", self.path.display()))?;
        for e in entries {
            let line = serde_json::to_string(e).map_err(|e| e.to_string())?;
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
