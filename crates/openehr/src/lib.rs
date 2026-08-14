//! openEHR adapter for Solum — EHRbase REST client (H3.0 Track B).
//!
//! Track B is **opt-in**: construct [`OpenEhrAdapter`] / [`EhrbaseClient`] with a
//! CDR base URL. When disabled (`cdr_url` is `None`), operations return
//! [`OpenEhrError::TrackBDisabled`].
//!
//! Pinned H3.0 template: [`PINNED_TEMPLATE_ID`] (`minimal_observation.en.v1`).
//! Fixture OPT: `fixtures/minimal_observation.opt`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Stage marker for roadmap / capability reporting.
pub const STAGE: &str = "3.1-fhir-aql";

/// Pinned operational template id for H3.0 façade smoke (see `VERSIONS`).
pub const PINNED_TEMPLATE_ID: &str = "minimal_observation.en.v1";

/// Embedded OPT bytes for the pinned template (upload via façade).
pub const PINNED_TEMPLATE_OPT: &str = include_str!("../fixtures/minimal_observation.opt");

/// Placeholder / config handle for a future or live openEHR CDR binding.
#[derive(Debug, Default, Clone)]
pub struct OpenEhrAdapter {
    pub cdr_url: Option<String>,
}

impl OpenEhrAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cdr_url(url: impl Into<String>) -> Self {
        Self {
            cdr_url: Some(url.into()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.cdr_url
            .as_ref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false)
    }

    /// Build an [`EhrbaseClient`] or return [`OpenEhrError::TrackBDisabled`].
    pub fn client(&self) -> Result<EhrbaseClient, OpenEhrError> {
        let Some(url) = self
            .cdr_url
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        else {
            return Err(OpenEhrError::TrackBDisabled);
        };
        EhrbaseClient::new(url)
    }
}

#[derive(Debug, Error)]
pub enum OpenEhrError {
    #[error("Track B CDR disabled (no --ehrbase-url / SOLUM_EHRBASE_URL)")]
    TrackBDisabled,
    #[error("AQL query rejected by Solum allowlist")]
    AqlRejected,
    #[error("EHRbase HTTP error: {0}")]
    Http(String),
    #[error("EHRbase returned status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("EHRbase response missing expected field: {0}")]
    MissingField(String),
    #[error(transparent)]
    Req(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Thin EHRbase openEHR REST client.
///
/// Base URL should include the `/ehrbase` context path when using the official
/// Docker image (e.g. `http://127.0.0.1:8081/ehrbase`).
#[derive(Debug, Clone)]
pub struct EhrbaseClient {
    base: String,
    http: reqwest::Client,
    user: Option<String>,
    password: Option<String>,
}

impl EhrbaseClient {
    pub fn new(base_url: &str) -> Result<Self, OpenEhrError> {
        let base = base_url.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return Err(OpenEhrError::TrackBDisabled);
        }
        let user = std::env::var("SOLUM_EHRBASE_USER")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let password = std::env::var("SOLUM_EHRBASE_PASSWORD").ok();
        let http = reqwest::Client::builder()
            .user_agent(format!("solum-openehr/{STAGE}"))
            .build()?;
        Ok(Self {
            base,
            http,
            user,
            password,
        })
    }

    fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.with_auth(self.http.get(url))
    }

    fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.with_auth(self.http.post(url))
    }

    fn with_auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match (&self.user, &self.password) {
            (Some(u), Some(p)) => rb.basic_auth(u, Some(p.as_str())),
            _ => rb,
        }
    }

    fn openehr_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/rest/openehr/v1/{path}", self.base)
    }

    /// Upload an ADL 1.4 operational template (XML OPT body).
    pub async fn upload_template_opt(&self, opt_xml: &str) -> Result<(), OpenEhrError> {
        let url = self.openehr_url("definition/template/adl1.4");
        let res = self
            .post(&url)
            // EHRbase 2.34 rejects Accept: application/json with 406 on OPT upload.
            .header("Content-Type", "application/xml")
            .header("Accept", "*/*")
            .header("Prefer", "return=minimal")
            .body(opt_xml.to_string())
            .send()
            .await?;
        let status = res.status();
        // 201 created, 204/200 ok, 409 already present — all acceptable for spike
        if status.is_success() || status.as_u16() == 409 {
            return Ok(());
        }
        let body = res.text().await.unwrap_or_default();
        Err(OpenEhrError::Status {
            status: status.as_u16(),
            body,
        })
    }

    /// Upload the pinned H3.0 OPT fixture.
    pub async fn ensure_pinned_template(&self) -> Result<(), OpenEhrError> {
        self.upload_template_opt(PINNED_TEMPLATE_OPT).await
    }

    /// Create an empty EHR; returns the EHR id (UUID string).
    pub async fn create_ehr(&self) -> Result<String, OpenEhrError> {
        let url = self.openehr_url("ehr");
        // EHRbase 2.34 requires a full EHR_STATUS when a JSON body is sent;
        // `{}` is rejected. Canonical status with PARTY_SELF works.
        let status_body = serde_json::json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": { "_type": "PARTY_SELF" },
            "is_modifiable": true,
            "is_queryable": true
        });
        let res = self
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Prefer", "return=representation")
            .json(&status_body)
            .send()
            .await?;
        let status = res.status();
        // Location header often carries the EHR id
        if let Some(loc) = res.headers().get(reqwest::header::LOCATION) {
            if let Ok(s) = loc.to_str() {
                if let Some(id) = s.rsplit('/').next() {
                    if !id.is_empty() && status.is_success() {
                        return Ok(id.to_string());
                    }
                }
            }
        }
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        if let Ok(v) = serde_json::from_str::<Value>(&body) {
            if let Some(id) = v
                .get("ehr_id")
                .and_then(|e| e.get("value"))
                .and_then(|v| v.as_str())
            {
                return Ok(id.to_string());
            }
        }
        Err(OpenEhrError::MissingField(
            "ehr_id (Location header or body)".into(),
        ))
    }

    /// Fetch EHRbase example composition (canonical JSON) for a template id.
    pub async fn example_composition(&self, template_id: &str) -> Result<Value, OpenEhrError> {
        let url = self.openehr_url(&format!(
            "definition/template/adl1.4/{}/example",
            urlencoding_path(template_id)
        ));
        let res = self
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }

    /// Fetch EHRbase example FLAT composition (may be incomplete on some EHRbase versions).
    pub async fn example_flat_composition(&self, template_id: &str) -> Result<Value, OpenEhrError> {
        let url = format!(
            "{}?format=FLAT",
            self.openehr_url(&format!(
                "definition/template/adl1.4/{}/example",
                urlencoding_path(template_id)
            ))
        );
        let res = self
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }

    /// Commit a canonical JSON composition; returns composition uid.
    pub async fn commit_composition(
        &self,
        ehr_id: &str,
        composition: &Value,
    ) -> Result<CompositionCommit, OpenEhrError> {
        let url = self.openehr_url(&format!("ehr/{ehr_id}/composition"));
        let res = self
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Prefer", "return=representation")
            .json(composition)
            .send()
            .await?;
        let status = res.status();
        let location = res
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let uid = composition_uid_from_response(location.as_deref(), &body)?;
        let template_id = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| {
                v.pointer("/archetype_details/template_id/value")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| PINNED_TEMPLATE_ID.to_string());
        Ok(CompositionCommit {
            ehr_id: ehr_id.to_string(),
            composition_uid: uid,
            template_id,
            body: serde_json::from_str(&body).unwrap_or(Value::Null),
        })
    }

    /// Commit a FLAT composition for `template_id`; returns composition uid.
    pub async fn commit_flat_composition(
        &self,
        ehr_id: &str,
        template_id: &str,
        composition: &Value,
    ) -> Result<CompositionCommit, OpenEhrError> {
        let url = format!(
            "{}?templateId={}&format=FLAT",
            self.openehr_url(&format!("ehr/{ehr_id}/composition")),
            urlencoding_query(template_id)
        );
        let res = self
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Prefer", "return=representation")
            .json(composition)
            .send()
            .await?;
        let status = res.status();
        let location = res
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let uid = composition_uid_from_response(location.as_deref(), &body)?;
        Ok(CompositionCommit {
            ehr_id: ehr_id.to_string(),
            composition_uid: uid,
            template_id: template_id.to_string(),
            body: serde_json::from_str(&body).unwrap_or(Value::Null),
        })
    }

    /// Get a composition by version uid (or object id).
    pub async fn get_composition(
        &self,
        ehr_id: &str,
        composition_uid: &str,
    ) -> Result<Value, OpenEhrError> {
        let url = self.openehr_url(&format!(
            "ehr/{ehr_id}/composition/{}",
            urlencoding_path(composition_uid)
        ));
        let res = self
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }

    /// Execute an AQL query against EHRbase (H3.1). Rejects non-allowlisted queries.
    pub async fn execute_aql(&self, aql: &str) -> Result<Value, OpenEhrError> {
        if !aql_allowed(aql) {
            return Err(OpenEhrError::AqlRejected);
        }
        let url = self.openehr_url("query/aql");
        let res = self
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "q": aql }))
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(OpenEhrError::Status {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }
}

/// H3.1 AQL allowlist: read-only SELECT over EHR/COMPOSITION; reject mutating tokens as words.
pub fn aql_allowed(aql: &str) -> bool {
    let trimmed = aql.trim();
    if trimmed.is_empty() || trimmed.len() > 8_192 {
        return false;
    }
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("SELECT") {
        return false;
    }
    if !upper.contains("COMPOSITION") && !upper.contains("EHR") {
        return false;
    }
    let stripped = strip_quoted(&upper);
    if stripped.contains(';') || stripped.contains("--") {
        return false;
    }
    const FORBIDDEN_WORDS: &[&str] = &[
        "DELETE", "DROP", "INSERT", "UPDATE", "TRUNCATE", "ALTER", "CREATE", "GRANT", "REVOKE",
        "EXECUTE", "CALL",
    ];
    for word in stripped.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if FORBIDDEN_WORDS.contains(&word) {
            return false;
        }
    }
    true
}

fn strip_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\'' || c == '"' {
            let quote = c;
            for d in chars.by_ref() {
                if d == quote {
                    break;
                }
            }
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionCommit {
    pub ehr_id: String,
    pub composition_uid: String,
    pub template_id: String,
    #[serde(default)]
    pub body: Value,
}

fn urlencoding_path(s: &str) -> String {
    // Composition UIDs contain `::` — encode for path safety.
    s.replace('%', "%25")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('#', "%23")
}

fn urlencoding_query(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
}

fn composition_uid_from_response(
    location: Option<&str>,
    body: &str,
) -> Result<String, OpenEhrError> {
    if let Some(loc) = location {
        if let Some(id) = loc.rsplit('/').next() {
            if !id.is_empty() {
                return Ok(id.to_string());
            }
        }
    }
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        if let Some(uid) = v
            .pointer("/uid/value")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/compositionUid").and_then(|x| x.as_str()))
            .or_else(|| {
                // FLAT often embeds "…/_uid"
                v.as_object().and_then(|m| {
                    m.iter()
                        .find(|(k, _)| k.ends_with("/_uid") || *k == "_uid")
                        .and_then(|(_, val)| val.as_str())
                })
            })
        {
            return Ok(uid.to_string());
        }
    }
    Err(OpenEhrError::MissingField(
        "composition uid (Location or body)".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Path,
        http::{HeaderMap, HeaderValue, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use std::net::SocketAddr;

    #[test]
    fn scaffold_disabled_by_default() {
        assert!(!OpenEhrAdapter::new().is_enabled());
        assert!(matches!(
            OpenEhrAdapter::new().client().unwrap_err(),
            OpenEhrError::TrackBDisabled
        ));
        assert_eq!(STAGE, "3.1-fhir-aql");
        assert_eq!(PINNED_TEMPLATE_ID, "minimal_observation.en.v1");
        assert!(PINNED_TEMPLATE_OPT.contains("minimal_observation"));
        assert!(aql_allowed(
            "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c"
        ));
        assert!(!aql_allowed("DELETE FROM EHR"));
        assert!(
            aql_allowed("SELECT c FROM EHR e CONTAINS COMPOSITION c WHERE c/name/value = 'Grant'"),
            "GRANT as a substring of a string/word must not false-reject"
        );
    }

    #[test]
    fn with_url_enables() {
        let a = OpenEhrAdapter::with_cdr_url("http://127.0.0.1:8081/ehrbase");
        assert!(a.is_enabled());
        assert!(a.client().is_ok());
    }

    async fn mock_ehrbase() -> (SocketAddr, tokio::task::JoinHandle<()>) {
        async fn create_ehr() -> impl IntoResponse {
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::LOCATION,
                HeaderValue::from_static(
                    "http://mock/ehrbase/rest/openehr/v1/ehr/11111111-1111-1111-1111-111111111111",
                ),
            );
            (StatusCode::CREATED, headers, Json(serde_json::json!({})))
        }

        async fn upload_template() -> StatusCode {
            StatusCode::CREATED
        }

        async fn example_canonical() -> impl IntoResponse {
            Json(serde_json::json!({
                "_type": "COMPOSITION",
                "name": { "value": "Minimal" },
                "archetype_details": {
                    "template_id": { "value": "minimal_observation.en.v1" }
                }
            }))
        }

        async fn commit_comp(Path(ehr_id): Path<String>) -> impl IntoResponse {
            let mut headers = HeaderMap::new();
            let loc = format!(
                "http://mock/ehrbase/rest/openehr/v1/ehr/{ehr_id}/composition/22222222-2222-2222-2222-222222222222::local.ehrbase.org::1"
            );
            headers.insert(
                axum::http::header::LOCATION,
                HeaderValue::from_str(&loc).unwrap(),
            );
            (
                StatusCode::CREATED,
                headers,
                Json(serde_json::json!({
                    "uid": { "value": "22222222-2222-2222-2222-222222222222::local.ehrbase.org::1" },
                    "archetype_details": {
                        "template_id": { "value": "minimal_observation.en.v1" }
                    }
                })),
            )
        }

        async fn get_comp(Path((_ehr_id, uid)): Path<(String, String)>) -> impl IntoResponse {
            Json(serde_json::json!({
                "uid": { "value": uid },
                "name": { "value": "Minimal observation" }
            }))
        }

        let app = Router::new()
            .route("/ehrbase/rest/openehr/v1/ehr", post(create_ehr))
            .route(
                "/ehrbase/rest/openehr/v1/definition/template/adl1.4",
                post(upload_template),
            )
            .route(
                "/ehrbase/rest/openehr/v1/definition/template/adl1.4/:id/example",
                get(example_canonical),
            )
            .route(
                "/ehrbase/rest/openehr/v1/ehr/:ehr_id/composition",
                post(commit_comp),
            )
            .route(
                "/ehrbase/rest/openehr/v1/ehr/:ehr_id/composition/:uid",
                get(get_comp),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (addr, handle)
    }

    #[tokio::test]
    async fn client_round_trip_against_mock() {
        let (addr, _handle) = mock_ehrbase().await;
        let base = format!("http://{addr}/ehrbase");
        let client = EhrbaseClient::new(&base).unwrap();

        client.ensure_pinned_template().await.expect("upload");
        let ehr_id = client.create_ehr().await.expect("create ehr");
        assert_eq!(ehr_id, "11111111-1111-1111-1111-111111111111");

        let example = client
            .example_composition(PINNED_TEMPLATE_ID)
            .await
            .expect("example");
        let commit = client
            .commit_composition(&ehr_id, &example)
            .await
            .expect("commit");
        assert!(commit.composition_uid.contains("22222222"));

        let got = client
            .get_composition(&ehr_id, &commit.composition_uid)
            .await
            .expect("get");
        assert_eq!(
            got.pointer("/uid/value").and_then(|v| v.as_str()),
            Some(commit.composition_uid.as_str())
        );
    }

    /// Live smoke against compose EHRbase — ignored by default.
    #[tokio::test]
    #[ignore = "requires Solum-Demo docker-compose.ehrbase.yml on :8081"]
    async fn live_ehrbase_smoke() {
        let client = EhrbaseClient::new("http://127.0.0.1:8081/ehrbase").unwrap();
        client.ensure_pinned_template().await.expect("upload opt");
        let ehr_id = client.create_ehr().await.expect("create ehr");
        let example = client
            .example_composition(PINNED_TEMPLATE_ID)
            .await
            .expect("example");
        let commit = client
            .commit_composition(&ehr_id, &example)
            .await
            .expect("commit");
        let _ = client
            .get_composition(&ehr_id, &commit.composition_uid)
            .await
            .expect("get");
    }
}
