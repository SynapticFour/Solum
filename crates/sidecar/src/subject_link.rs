//! File-backed subject bridge store (H3.3) — Crypt4GH JSONL.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use solum_core::crypto::{Crypt4ghKeyProvider, EncryptedField, KeyRef};

use crate::store_crypto::{decrypt_store_json, encrypt_store_json, SUBJECT_LINK_CATEGORY};

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
    current: HashMap<String, SubjectLink>,
    key_ref: KeyRef,
    categories: Vec<String>,
}

impl SubjectLinkStore {
    pub fn open(
        path: impl AsRef<Path>,
        provider: &impl Crypt4ghKeyProvider,
        key_ref: KeyRef,
        categories: Vec<String>,
    ) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("subject-link mkdir {}: {e}", parent.display()))?;
        }
        let mut current = HashMap::new();
        if path.exists() {
            let file = File::open(&path)
                .map_err(|e| format!("subject-link open {}: {e}", path.display()))?;
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|e| format!("subject-link read: {e}"))?;
                if line.trim().is_empty() {
                    continue;
                }
                let field: EncryptedField = serde_json::from_str(&line).map_err(|e| {
                    format!(
                        "subject-link: plaintext or corrupt line (Crypt4GH envelope required): {e}"
                    )
                })?;
                let link: SubjectLink = decrypt_store_json(provider, &field, &key_ref)?;
                current.insert(link.solum_subject_id.clone(), link);
            }
        } else {
            File::create(&path)
                .map_err(|e| format!("subject-link create {}: {e}", path.display()))?;
        }
        Ok(Self {
            path,
            current,
            key_ref,
            categories,
        })
    }

    pub fn upsert(
        &mut self,
        provider: &impl Crypt4ghKeyProvider,
        link: &SubjectLink,
    ) -> Result<(), String> {
        let field = encrypt_store_json(
            provider,
            &self.categories,
            &self.key_ref,
            SUBJECT_LINK_CATEGORY,
            link,
        )?;
        let line = serde_json::to_string(&field).map_err(|e| e.to_string())?;
        crate::store_crypto::prepare_jsonl_append(&self.path, line.len() as u64 + 1)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("subject-link append {}: {e}", self.path.display()))?;
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        self.current
            .insert(link.solum_subject_id.clone(), link.clone());
        Ok(())
    }

    pub fn get(&self, solum_subject_id: &str) -> Result<Option<SubjectLink>, String> {
        Ok(self.current.get(solum_subject_id).cloned())
    }
}
