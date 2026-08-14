//! Size cap and rotation for façade JSONL files (FHIR, subject-link, dead-letter).
//!
//! Profile `retention.*_days` is a policy floor, not a file-rotation engine.
//! These helpers cap unbounded growth: rotate the live file when an append
//! would exceed [`jsonl_max_bytes`] (default 256 MiB, override
//! `SOLUM_JSONL_MAX_BYTES`). Rotated files keep the original name plus a
//! unix-seconds suffix. Operators must archive or delete rotated segments.

use std::fs::{self, File};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default cap for a single live JSONL segment (256 MiB).
pub const DEFAULT_JSONL_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Live-file byte cap. `SOLUM_JSONL_MAX_BYTES` when set and > 0, else the default.
pub fn jsonl_max_bytes() -> u64 {
    std::env::var("SOLUM_JSONL_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_JSONL_MAX_BYTES)
}

/// If `path` plus `extra` would exceed `max_bytes`, rename the live file and
/// create an empty replacement so the append can proceed.
pub fn rotate_jsonl_if_needed_with_max(
    path: &Path,
    extra: u64,
    max_bytes: u64,
) -> Result<(), String> {
    if extra > max_bytes {
        return Err(format!(
            "JSONL record ({} bytes) exceeds SOLUM_JSONL_MAX_BYTES ({max_bytes})",
            extra
        ));
    }
    let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if len.saturating_add(extra) <= max_bytes {
        return Ok(());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("JSONL rotate: invalid path {}", path.display()))?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let rotated = path.with_file_name(format!("{name}.{ts}"));
    fs::rename(path, &rotated).map_err(|e| {
        format!(
            "JSONL rotate {} → {}: {e}",
            path.display(),
            rotated.display()
        )
    })?;
    File::create(path).map_err(|e| format!("JSONL recreate {}: {e}", path.display()))?;
    Ok(())
}

/// [`rotate_jsonl_if_needed_with_max`] using [`jsonl_max_bytes`].
pub fn rotate_jsonl_if_needed(path: &Path, extra: u64) -> Result<(), String> {
    rotate_jsonl_if_needed_with_max(path, extra, jsonl_max_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rotates_when_cap_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "aaaaaaaaaa").unwrap();
        drop(f);
        rotate_jsonl_if_needed_with_max(&path, 8, 16).unwrap();
        let live = fs::read_to_string(&path).unwrap();
        assert!(live.is_empty(), "live file should be empty after rotate");
        let siblings: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("store.jsonl."))
            .collect();
        assert_eq!(
            siblings.len(),
            1,
            "expected one rotated segment: {siblings:?}"
        );
    }

    #[test]
    fn refuses_single_record_larger_than_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.jsonl");
        File::create(&path).unwrap();
        let err = rotate_jsonl_if_needed_with_max(&path, 32, 16).unwrap_err();
        assert!(err.contains("exceeds"), "{err}");
    }
}
