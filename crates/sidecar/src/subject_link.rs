//! File-backed subject bridge store (H3.3) — JSONL.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectLink {
    /// Canonical Solum clinical subject pseudonym.
    pub solum_subject_id: String,
    /// Optional Ferrum DRS object id (genomic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ferrum_drs_id: Option<String>,
    /// Optional BRA Phenopacket id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phenopacket_id: Option<String>,
    /// Optional openEHR EHR id when Track B is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ehr_id: Option<String>,
}

#[derive(Debug)]
pub struct SubjectLinkStore {
    path: PathBuf,
}

impl SubjectLinkStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("subject-link mkdir {}: {e}", parent.display()))?;
        }
        if !path.exists() {
            File::create(&path)
                .map_err(|e| format!("subject-link create {}: {e}", path.display()))?;
        }
        Ok(Self { path })
    }

    pub fn upsert(&self, link: &SubjectLink) -> Result<(), String> {
        let mut keep = self.read_all()?;
        keep.retain(|e| e.solum_subject_id != link.solum_subject_id);
        keep.push(link.clone());
        self.rewrite_all(&keep)
    }

    pub fn get(&self, solum_subject_id: &str) -> Result<Option<SubjectLink>, String> {
        Ok(self
            .read_all()?
            .into_iter()
            .find(|e| e.solum_subject_id == solum_subject_id))
    }

    fn read_all(&self) -> Result<Vec<SubjectLink>, String> {
        let file = File::open(&self.path)
            .map_err(|e| format!("subject-link open {}: {e}", self.path.display()))?;
        let mut out = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("subject-link read: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(&line).map_err(|e| format!("subject-link parse: {e}"))?);
        }
        Ok(out)
    }

    fn rewrite_all(&self, entries: &[SubjectLink]) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| format!("subject-link rewrite {}: {e}", self.path.display()))?;
        for e in entries {
            let line = serde_json::to_string(e).map_err(|e| e.to_string())?;
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
