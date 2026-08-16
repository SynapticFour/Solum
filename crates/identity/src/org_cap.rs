//! Org IAM: map OIDC group claim values → Solum `CAP_*` strings (H2.2).
//!
//! Pattern mirrors ga4gh-infra ADS `claim_path` / `claim_value` matching, but
//! targets Solum capabilities instead of dataset UUID grants.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

/// One claim-value → capabilities row.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OrgCapMapEntry {
    pub claim_value: String,
    pub capabilities: Vec<String>,
}

/// TOML org-IAM mapping file.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OrgCapMapping {
    /// JWT claim path (e.g. `groups` or `realm_access.roles`).
    #[serde(default = "default_claim_path")]
    pub claim_path: String,
    #[serde(default, rename = "map")]
    pub entries: Vec<OrgCapMapEntry>,
}

fn default_claim_path() -> String {
    "groups".into()
}

impl OrgCapMapping {
    /// Load mapping from a TOML file.
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read org-iam config {}: {e}", path.display()))?;
        Self::from_toml_str(&raw)
            .map_err(|e| format!("invalid org-iam TOML {}: {e}", path.display()))
    }

    /// Parse mapping from a TOML string.
    pub fn from_toml_str(raw: &str) -> Result<Self, String> {
        toml::from_str(raw).map_err(|e| e.to_string())
    }

    /// Union of capabilities for all matching claim values (deduped, sorted).
    pub fn resolve_capabilities(&self, claim_values: &[String]) -> Vec<String> {
        let mut out = BTreeSet::new();
        for value in claim_values {
            for entry in &self.entries {
                if &entry.claim_value == value {
                    for cap in &entry.capabilities {
                        if !cap.is_empty() {
                            out.insert(cap.clone());
                        }
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

/// Extract string claim values from a JWT claims object (ADS-style path).
///
/// Supports top-level keys and dotted paths (`realm_access.roles`). Values may
/// be a string or an array of strings.
pub fn claim_values_from_json(claims: &serde_json::Value, path: &str) -> Vec<String> {
    match claims.as_object() {
        Some(map) => claim_values_from_map(map, path),
        None => vec![],
    }
}

/// Same as [`claim_values_from_json`] without wrapping a `Map` in a `Value`.
pub fn claim_values_from_map(
    map: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Vec<String> {
    if path.is_empty() {
        return vec![];
    }
    if let Some(value) = map.get(path) {
        return values_from_json(value);
    }
    if path.contains('.') {
        let mut segs = path.split('.');
        let Some(first) = segs.next() else {
            return vec![];
        };
        let mut current = map.get(first);
        for seg in segs {
            current = current.and_then(|v| v.get(seg));
        }
        return current.map(values_from_json).unwrap_or_default();
    }
    vec![]
}

fn values_from_json(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        serde_json::Value::Bool(b) => vec![b.to_string()],
        serde_json::Value::Number(n) => vec![n.to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_union_and_dedupe() {
        let mapping = OrgCapMapping::from_toml_str(
            r#"
claim_path = "groups"
[[map]]
claim_value = "solum-consent-ops"
capabilities = ["solum:consent:grant", "solum:consent:revoke"]
[[map]]
claim_value = "solum-crypto-ops"
capabilities = ["solum:crypto:encrypt", "solum:consent:grant"]
"#,
        )
        .unwrap();
        let caps =
            mapping.resolve_capabilities(&["solum-consent-ops".into(), "solum-crypto-ops".into()]);
        assert_eq!(
            caps,
            vec![
                "solum:consent:grant".to_string(),
                "solum:consent:revoke".to_string(),
                "solum:crypto:encrypt".to_string(),
            ]
        );
    }

    #[test]
    fn miss_yields_empty() {
        let mapping = OrgCapMapping::from_toml_str(
            r#"
[[map]]
claim_value = "solum-consent-ops"
capabilities = ["solum:consent:grant"]
"#,
        )
        .unwrap();
        assert!(mapping
            .resolve_capabilities(&["other-group".into()])
            .is_empty());
    }

    #[test]
    fn claim_values_array_and_dotted() {
        let claims = json!({
            "groups": ["ega-approved", "staff"],
            "realm_access": { "roles": ["researcher", "admin"] }
        });
        assert_eq!(
            claim_values_from_json(&claims, "groups"),
            vec!["ega-approved", "staff"]
        );
        assert_eq!(
            claim_values_from_json(&claims, "realm_access.roles"),
            vec!["researcher", "admin"]
        );
    }
}
