//! File-backed FHIR resource store for H3.1 façade (JSONL, append-only).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solum_core::fhir::{fhir_resource_type_allowed, ALLOWED_FHIR_RESOURCE_TYPES};

/// IPS-aligned resource types allowed on the H3.1 façade.
pub const ALLOWED_FHIR_TYPES: &[&str] = ALLOWED_FHIR_RESOURCE_TYPES;

pub fn fhir_type_allowed(resource_type: &str) -> bool {
    fhir_resource_type_allowed(resource_type)
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
    current: HashMap<String, HashMap<String, StoredFhirResource>>,
}

impl FhirStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("fhir store mkdir {}: {e}", parent.display()))?;
        }
        let mut current: HashMap<String, HashMap<String, StoredFhirResource>> = HashMap::new();
        if path.exists() {
            let file = File::open(&path)
                .map_err(|e| format!("fhir store open {}: {e}", path.display()))?;
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|e| format!("fhir store read: {e}"))?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: StoredFhirResource =
                    serde_json::from_str(&line).map_err(|e| format!("fhir store parse: {e}"))?;
                current
                    .entry(entry.resource_type.clone())
                    .or_default()
                    .insert(entry.id.clone(), entry);
            }
        } else {
            File::create(&path)
                .map_err(|e| format!("fhir store create {}: {e}", path.display()))?;
        }
        Ok(Self { path, current })
    }

    pub fn upsert(&mut self, entry: &StoredFhirResource) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("fhir store append {}: {e}", self.path.display()))?;
        let line = serde_json::to_string(entry).map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        self.current
            .entry(entry.resource_type.clone())
            .or_default()
            .insert(entry.id.clone(), entry.clone());
        Ok(())
    }

    pub fn get(&self, resource_type: &str, id: &str) -> Result<Option<StoredFhirResource>, String> {
        Ok(self
            .current
            .get(resource_type)
            .and_then(|m| m.get(id))
            .cloned())
    }
}
