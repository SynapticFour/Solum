//! Persistent, tamper-evident audit storage.
//!
//! [`crate::AuditLog`] is in-memory only and loses everything on process
//! exit — not sufficient for Annex II "comprehensive access log" retention
//! (the `eu-ehds` profile requires a 10-year audit floor). [`FileAuditStore`]
//! appends every event to a file as a hash-chained JSON-lines log: each
//! record's hash covers its own content plus the previous record's hash, so
//! any edit, deletion, or reordering after the fact is detectable by
//! [`FileAuditStore::verify_chain`].
//!
//! This is **tamper-evidence**, not a cryptographic signature — there is no
//! private key and no non-repudiation claim. Signing / attestation is
//! HELIOS's job (see `docs/helios.md`); this store produces the durable,
//! independently-verifiable trail that a HELIOS-class tool would sign over.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::AuditEvent;

/// Hash that precedes the first record in a chain (64 hex zero characters).
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Error)]
pub enum AuditStoreError {
    #[error("audit store I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("audit store serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("audit store corrupt at {path}:{line}: {reason}")]
    Corrupt {
        path: String,
        line: usize,
        reason: String,
    },
    #[error("audit chain broken at seq {seq}: {reason}")]
    ChainBroken { seq: u64, reason: String },
}

/// One persisted, hash-chained audit record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Monotonically increasing sequence number, starting at 1.
    pub seq: u64,
    pub event: AuditEvent,
    /// Hash of the previous record (or [`GENESIS_HASH`] for the first record).
    pub prev_hash: String,
    /// SHA-256 over `seq || prev_hash || canonical(event)`, hex-encoded.
    pub hash: String,
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn compute_hash(seq: u64, prev_hash: &str, event: &AuditEvent) -> Result<String, AuditStoreError> {
    // `AuditEvent`'s field order is fixed by its struct definition, so
    // `serde_json::to_string` is stable enough for a same-process /
    // same-schema-version hash chain. This is not a general-purpose
    // canonicalization guarantee across arbitrary JSON producers.
    let event_json = serde_json::to_string(event)?;
    let mut hasher = Sha256::new();
    hasher.update(seq.to_be_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update(event_json.as_bytes());
    Ok(to_hex(&hasher.finalize()))
}

fn verify_records(records: &[AuditRecord]) -> Result<(), AuditStoreError> {
    let mut expected_prev = GENESIS_HASH.to_string();
    for (idx, record) in records.iter().enumerate() {
        let expected_seq = idx as u64 + 1;
        if record.seq != expected_seq {
            return Err(AuditStoreError::ChainBroken {
                seq: record.seq,
                reason: format!("expected seq {expected_seq}, found {}", record.seq),
            });
        }
        if record.prev_hash != expected_prev {
            return Err(AuditStoreError::ChainBroken {
                seq: record.seq,
                reason: "prev_hash does not match preceding record".into(),
            });
        }
        let recomputed = compute_hash(record.seq, &record.prev_hash, &record.event)?;
        if recomputed != record.hash {
            return Err(AuditStoreError::ChainBroken {
                seq: record.seq,
                reason: "stored hash does not match recomputed hash — record was altered".into(),
            });
        }
        expected_prev = record.hash.clone();
    }
    Ok(())
}

/// Append-only, hash-chained audit log backed by a single file.
///
/// Not concurrency-safe across processes (single-writer assumption for
/// stage 1; a durable multi-writer backend is stage-2 scope — see
/// `docs/roadmap.md`).
#[derive(Debug)]
pub struct FileAuditStore {
    path: PathBuf,
    last_seq: u64,
    last_hash: String,
}

impl FileAuditStore {
    /// Open (or create) an append-only audit log at `path`, recovering chain
    /// state (last sequence number + last hash) from any existing content.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuditStoreError> {
        let path = path.as_ref().to_path_buf();
        let (last_seq, last_hash) = if path.exists() {
            let records = Self::read_records(&path)?;
            verify_records(&records)?;
            match records.last() {
                Some(r) => (r.seq, r.hash.clone()),
                None => (0, GENESIS_HASH.to_string()),
            }
        } else {
            (0, GENESIS_HASH.to_string())
        };
        Ok(Self {
            path,
            last_seq,
            last_hash,
        })
    }

    fn read_records(path: &Path) -> Result<Vec<AuditRecord>, AuditStoreError> {
        let file = File::open(path).map_err(|source| AuditStoreError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for (idx, line) in reader.lines().enumerate() {
            let line = line.map_err(|source| AuditStoreError::Io {
                path: path.display().to_string(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: AuditRecord =
                serde_json::from_str(&line).map_err(|source| AuditStoreError::Corrupt {
                    path: path.display().to_string(),
                    line: idx + 1,
                    reason: source.to_string(),
                })?;
            records.push(record);
        }
        Ok(records)
    }

    /// Append one event to the chain. Fsync'd before returning, so a
    /// successful return means the record survives a process crash.
    pub fn append(&mut self, event: AuditEvent) -> Result<AuditRecord, AuditStoreError> {
        let seq = self.last_seq + 1;
        let hash = compute_hash(seq, &self.last_hash, &event)?;
        let record = AuditRecord {
            seq,
            event,
            prev_hash: self.last_hash.clone(),
            hash: hash.clone(),
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| AuditStoreError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|source| AuditStoreError::Io {
                path: self.path.display().to_string(),
                source,
            })?;

        self.last_seq = seq;
        self.last_hash = hash;
        Ok(record)
    }

    /// Read every record back, in append order. Does **not** verify the
    /// chain — use [`Self::verify_chain`] for that.
    pub fn read_all(&self) -> Result<Vec<AuditRecord>, AuditStoreError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        Self::read_records(&self.path)
    }

    /// Replay the full chain from disk and confirm no record was altered,
    /// reordered, or deleted since it was appended. This is the "log
    /// review" capability Annex II expects an operator to be able to run.
    ///
    /// [`Self::open`] already runs this check; calling it again re-reads the file.
    pub fn verify_chain(&self) -> Result<(), AuditStoreError> {
        let records = self.read_all()?;
        verify_records(&records)
    }

    /// Export the full chain as a HELIOS-oriented JSON envelope. The chain
    /// (`prev_hash`/`hash` per record) is included so a downstream evidence
    /// tool can verify integrity without re-deriving it from `AuditEvent`
    /// alone.
    pub fn export_helios_json(&self) -> Result<String, AuditStoreError> {
        let records = self.read_all()?;
        let envelope = HeliosChainEnvelope {
            format: "solum-audit-helios-chain-v1",
            generator: "solum-audit",
            record_count: records.len(),
            records,
        };
        Ok(serde_json::to_string_pretty(&envelope)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeliosChainEnvelope {
    format: &'static str,
    generator: &'static str,
    record_count: usize,
    records: Vec<AuditRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditOutcome;
    use chrono::Utc;

    fn sample_event(kind: &str) -> AuditEvent {
        AuditEvent {
            event_type: kind.into(),
            timestamp: Utc::now(),
            actor: "practitioner/123".into(),
            data_category: Some("patient_summary".into()),
            outcome: AuditOutcome::Success,
            details: Default::default(),
        }
    }

    #[test]
    fn appends_and_chains_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileAuditStore::open(&path).unwrap();

        let r1 = store.append(sample_event("access.granted")).unwrap();
        let r2 = store.append(sample_event("data.read")).unwrap();

        assert_eq!(r1.seq, 1);
        assert_eq!(r1.prev_hash, GENESIS_HASH);
        assert_eq!(r2.seq, 2);
        assert_eq!(r2.prev_hash, r1.hash);
        assert!(store.verify_chain().is_ok());
    }

    #[test]
    fn recovers_chain_state_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        {
            let mut store = FileAuditStore::open(&path).unwrap();
            store.append(sample_event("access.granted")).unwrap();
        }

        let mut store = FileAuditStore::open(&path).unwrap();
        let r2 = store.append(sample_event("data.read")).unwrap();
        assert_eq!(r2.seq, 2, "sequence must continue across reopen");
        assert!(store.verify_chain().is_ok());
    }

    #[test]
    fn detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileAuditStore::open(&path).unwrap();
        store.append(sample_event("access.granted")).unwrap();
        store.append(sample_event("data.read")).unwrap();

        // Tamper: rewrite the file with an altered actor on the first record.
        let contents = std::fs::read_to_string(&path).unwrap();
        let tampered = contents.replacen("practitioner/123", "practitioner/999", 1);
        std::fs::write(&path, tampered).unwrap();

        let err = FileAuditStore::open(&path).expect_err("tampering must be detected on open");
        assert!(matches!(err, AuditStoreError::ChainBroken { .. }));
    }

    #[test]
    fn detects_deleted_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileAuditStore::open(&path).unwrap();
        store.append(sample_event("access.granted")).unwrap();
        store.append(sample_event("data.read")).unwrap();
        store.append(sample_event("data.export")).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        // Drop the middle record, keep first and last — seq/prev_hash both break.
        let tampered = format!("{}\n{}\n", lines[0], lines[2]);
        std::fs::write(&path, tampered).unwrap();

        let err = FileAuditStore::open(&path).expect_err("deleted record must be detected on open");
        assert!(matches!(err, AuditStoreError::ChainBroken { .. }));
    }

    #[test]
    fn exports_helios_chain_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = FileAuditStore::open(&path).unwrap();
        store.append(sample_event("access.granted")).unwrap();

        let json = store.export_helios_json().unwrap();
        assert!(json.contains("solum-audit-helios-chain-v1"));
        assert!(json.contains("access.granted"));
    }
}
