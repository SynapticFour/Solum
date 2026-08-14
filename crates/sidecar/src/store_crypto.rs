//! Crypt4GH envelopes for façade JSONL lines (FHIR / subject-link / dead-letter).

use serde::de::DeserializeOwned;
use serde::Serialize;
use solum_core::crypto::{
    decrypt_field, encrypt_field, Crypt4ghKeyProvider, EncryptedField, FieldCategoryGate, KeyRef,
};
use solum_core::rotate_jsonl_if_needed;
use std::path::Path;

pub const FHIR_STORE_CATEGORY: &str = "fhir_resource";
pub const SUBJECT_LINK_CATEGORY: &str = "subject_link";

pub fn prepare_jsonl_append(path: &Path, extra: u64) -> Result<(), String> {
    rotate_jsonl_if_needed(path, extra)
}

pub fn encrypt_store_json(
    provider: &impl Crypt4ghKeyProvider,
    categories: &[String],
    key_ref: &KeyRef,
    category: &str,
    value: &impl Serialize,
) -> Result<EncryptedField, String> {
    let gate = FieldCategoryGate::new(categories);
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    encrypt_field(&gate, provider, category, &bytes, key_ref).map_err(|e| e.to_string())
}

pub fn decrypt_store_json<T: DeserializeOwned>(
    provider: &impl Crypt4ghKeyProvider,
    field: &EncryptedField,
    key_ref: &KeyRef,
) -> Result<T, String> {
    let bytes = decrypt_field(provider, field, key_ref).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
